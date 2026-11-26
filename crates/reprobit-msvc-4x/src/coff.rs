//! The parts of an i386 COFF object a schedule reads and moves: one
//! function's bytes, its relocations, and the offsets inside it that other
//! records point at, which must keep their place.

use std::collections::BTreeSet;

use anyhow::{Context, Result, bail, ensure};

const SYMBOL: usize = 18;
const RELOCATION: usize = 10;
/// `IMAGE_REL_I386_SECTION` holds a section number, not an address.
const SECTION_NUMBER: u16 = 0x0a;
/// Procedure records whose body holds the debug start, end, and length.
const PROCS: [u16; 6] = [0x0204, 0x0205, 0x100a, 0x100b, 0x110f, 0x1110];
/// Block records whose body holds a length at +8.
const BLOCKS: [u16; 2] = [0x0207, 0x1103];

fn u16_at(bytes: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        bytes
            .get(at..at + 2)
            .context("the object is truncated")?
            .try_into()?,
    ))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<usize> {
    Ok(usize::try_from(u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .context("the object is truncated")?
            .try_into()?,
    ))?)
}

/// A section header's fields.
struct Section {
    name: String,
    size: usize,
    data: usize,
    relocations: usize,
    count: usize,
    lines: usize,
    line_count: usize,
}

/// A symbol table entry's fields.
struct Symbol {
    name: String,
    value: usize,
    /// One-based section number; zero or negative for none.
    section: i16,
    class: u8,
}

/// A parsed object.
pub(crate) struct Coff<'a> {
    bytes: &'a [u8],
    sections: Vec<Section>,
    symbols: Vec<Option<Symbol>>,
}

/// A relocation inside the function.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Relocation {
    /// File offset of the relocation's table entry.
    pub(crate) entry: usize,
    /// Offset within the function.
    pub(crate) offset: usize,
    pub(crate) target: String,
    pub(crate) kind: u16,
}

/// One function located in an object.
pub(crate) struct Function {
    /// File offset of the function's first byte.
    pub(crate) file: usize,
    /// Offset of the function's first byte within its section.
    pub(crate) start: usize,
    pub(crate) bytes: Vec<u8>,
    pub(crate) relocations: Vec<Relocation>,
    /// Offsets within the function that something outside its code names.
    pub(crate) barriers: BTreeSet<usize>,
}

impl Function {
    /// The offsets within the function of its relocated fields.
    pub(crate) fn fields(&self) -> Vec<usize> {
        self.relocations
            .iter()
            .map(|relocation| relocation.offset)
            .collect()
    }
}

impl<'a> Coff<'a> {
    /// Parses an i386 COFF object.
    pub(crate) fn parse(bytes: &'a [u8]) -> Result<Self> {
        ensure!(u16_at(bytes, 0)? == 0x14c, "the object is not i386 COFF");
        let (symbols_at, count) = (u32_at(bytes, 8)?, u32_at(bytes, 12)?);
        let strings = symbols_at + count * SYMBOL;
        let text = |raw: &[u8]| {
            String::from_utf8_lossy(
                &raw[..raw.iter().position(|&byte| byte == 0).unwrap_or(raw.len())],
            )
            .into_owned()
        };
        let string = |offset: usize| -> Result<String> {
            Ok(text(
                bytes
                    .get(strings + offset..)
                    .context("a name lies outside the string table")?,
            ))
        };
        let name = |at: usize| -> Result<String> {
            let raw = bytes.get(at..at + 8).context("the object is truncated")?;
            if raw[..4] == [0; 4] {
                string(u32_at(raw, 4)?)
            } else {
                Ok(text(raw))
            }
        };
        let first = 20 + usize::from(u16_at(bytes, 16)?);
        let sections = (0..usize::from(u16_at(bytes, 2)?))
            .map(|index| {
                let at = first + index * 40;
                let flags = u32_at(bytes, at + 36)?;
                ensure!(
                    flags & 0x0100_0000 == 0,
                    "a section has more relocations than COFF counts"
                );
                // A long section name is `/offset` into the string table.
                let short = name(at)?;
                let name = match short.strip_prefix('/').map(str::parse::<usize>) {
                    Some(Ok(offset)) => string(offset)?,
                    _ => short,
                };
                Ok(Section {
                    name,
                    size: u32_at(bytes, at + 16)?,
                    data: u32_at(bytes, at + 20)?,
                    relocations: u32_at(bytes, at + 24)?,
                    count: usize::from(u16_at(bytes, at + 32)?),
                    lines: u32_at(bytes, at + 28)?,
                    line_count: usize::from(u16_at(bytes, at + 34)?),
                })
            })
            .collect::<Result<_>>()?;
        let mut symbols = Vec::with_capacity(count);
        while symbols.len() < count {
            let at = symbols_at + symbols.len() * SYMBOL;
            symbols.push(Some(Symbol {
                name: name(at)?,
                value: u32_at(bytes, at + 8)?,
                section: i16::from_le_bytes(u16_at(bytes, at + 12)?.to_le_bytes()),
                class: bytes[at + 16],
            }));
            // Auxiliary records take symbol indices but name nothing.
            symbols.extend((0..bytes[at + 17]).map(|_| None));
        }
        Ok(Self {
            bytes,
            sections,
            symbols,
        })
    }

    fn symbol(&self, index: usize) -> Result<&Symbol> {
        self.symbols
            .get(index)
            .and_then(Option::as_ref)
            .context("a relocation names a missing symbol")
    }

    /// Each relocation of section `index` as `(entry, offset, symbol, kind)`.
    fn relocations(&self, index: usize) -> Result<Vec<(usize, usize, usize, u16)>> {
        let section = &self.sections[index];
        (0..section.count)
            .map(|item| {
                let entry = section.relocations + item * RELOCATION;
                Ok((
                    entry,
                    u32_at(self.bytes, entry)?,
                    u32_at(self.bytes, entry + 4)?,
                    u16_at(self.bytes, entry + 8)?,
                ))
            })
            .collect()
    }

    /// Locates `name`. Its extent runs to the next symbol in its section, or
    /// the section's end, as `rbit` reads it.
    pub(crate) fn function(&self, name: &str) -> Result<Function> {
        let found = self
            .symbols
            .iter()
            .flatten()
            .find(|symbol| symbol.name == name)
            .with_context(|| format!("the object has no symbol {name}"))?;
        let number = found.section;
        let index = usize::try_from(number)
            .ok()
            .and_then(|number| number.checked_sub(1))
            .context("the symbol is not defined in a section")?;
        let section = &self.sections[index];
        let start = found.value;
        let end = self
            .symbols
            .iter()
            .flatten()
            // Classes 100 and 101 are `.bb`, `.bf`, and their kin, which mark places inside a function.
            .filter(|symbol| {
                symbol.section == number
                    && !matches!(symbol.class, 100 | 101)
                    && symbol.value > start
            })
            .map(|symbol| symbol.value)
            .min()
            .unwrap_or(section.size);
        let bytes = self
            .bytes
            .get(section.data + start..section.data + end)
            .context("the symbol lies outside its section")?
            .to_vec();
        let relocations = self
            .relocations(index)?
            .into_iter()
            .filter(|&(_, offset, ..)| (start..end).contains(&offset))
            .map(|(entry, offset, symbol, kind)| {
                Ok(Relocation {
                    entry,
                    offset: offset - start,
                    target: self.symbol(symbol)?.name.clone(),
                    kind,
                })
            })
            .collect::<Result<_>>()?;
        let mut function = Function {
            file: section.data + start,
            start,
            bytes,
            relocations,
            barriers: BTreeSet::new(),
        };
        let inside = |offset: usize| (start < offset && offset < end).then(|| offset - start);
        // Line numbers: an address, or a symbol index where the line is 0.
        for line in (0..section.line_count).map(|item| section.lines + item * 6) {
            if u16_at(self.bytes, line + 4)? != 0 {
                function.barriers.extend(inside(u32_at(self.bytes, line)?));
            }
        }
        for (other, header) in self.sections.iter().enumerate() {
            // Each place another record points into the function, by symbol plus in-place addend.
            let mut targets = Vec::new();
            for (_, offset, symbol, kind) in self.relocations(other)? {
                let symbol = self.symbol(symbol)?;
                if symbol.section != number || kind == SECTION_NUMBER {
                    continue;
                }
                let target = symbol.value + u32_at(self.bytes, header.data + offset)?;
                if (start..=end).contains(&target) {
                    function.barriers.extend(inside(target));
                    targets.push((offset, target));
                }
            }
            if targets.is_empty() {
                continue;
            }
            let data = self
                .bytes
                .get(header.data..header.data + header.size)
                .context("a section lies outside the object")?;
            match header.name.as_str() {
                ".debug$S" => records(data, &targets, &mut |at| {
                    function.barriers.extend(inside(at));
                })?,
                // FPO data: 16-byte entries whose byte at +14 is the prologue's length.
                ".debug$F" => {
                    for &(offset, target) in &targets {
                        function.barriers.extend(inside(
                            target
                                + usize::from(
                                    *data.get(offset + 14).context("FPO data is truncated")?,
                                ),
                        ));
                    }
                }
                name if name.starts_with(".debug") => bail!(
                    "{name} describes the function, and only CodeView 4 and FPO data are kept in step"
                ),
                _ => {}
            }
        }
        Ok(function)
    }
}

/// Adds the places `CodeView` 4 procedure and block records name, from
/// each record's relocated address and its offsets from it.
fn records(data: &[u8], targets: &[(usize, usize)], barrier: &mut dyn FnMut(usize)) -> Result<()> {
    let mut at = match u32_at(data, 0)? {
        1 | 2 => 4,
        // A COMDAT function's own section holds records without a signature.
        first if first > 0xffff => 0,
        _ => bail!("the function's CodeView is not version 4, and only version 4 is kept in step"),
    };
    while at + 4 <= data.len() {
        let next = at + 2 + usize::from(u16_at(data, at)?);
        let (kind, body) = (u16_at(data, at + 2)?, at + 4);
        if let Some(&(_, target)) = targets
            .iter()
            .find(|(offset, _)| (at..next).contains(offset))
        {
            if PROCS.contains(&kind) {
                for field in [12, 16, 20] {
                    barrier(target + u32_at(data, body + field)?);
                }
            } else if BLOCKS.contains(&kind) {
                barrier(target + u32_at(data, body + 8)?);
            }
        }
        at = next;
    }
    Ok(())
}
