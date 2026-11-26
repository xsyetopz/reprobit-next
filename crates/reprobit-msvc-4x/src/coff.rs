//! The parts of an i386 COFF object the rewrites read and move: one
//! function's bytes, its relocations, and the offsets inside it that other
//! records point at, and the object rebuilt around new code for it.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};

const SYMBOL: usize = 18;
const RELOCATION: usize = 10;
/// `IMAGE_REL_I386_SECTION` holds a section number, not an address.
const SECTION_NUMBER: u16 = 0x0a;
/// Procedure records whose body holds the debug start, end, and length.
const PROCS: [u16; 6] = [0x0204, 0x0205, 0x100a, 0x100b, 0x110f, 0x1110];
/// Block records whose body holds a length at +8.
const BLOCKS: [u16; 2] = [0x0207, 0x1103];
/// Relocation types whose field holds an in-place addend to a symbol's
/// address: `DIR32`, `DIR32NB`, `SECREL`, and `REL32`.
const ADDRESSED: [u16; 4] = [0x06, 0x07, 0x0b, 0x14];

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

/// Relocations' offsets and the places they name.
type Targets = Vec<(usize, usize)>;

/// A section header's fields.
struct Section {
    /// File offset of the header.
    at: usize,
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
    /// File offset of the entry.
    at: usize,
    name: String,
    value: usize,
    /// One-based section number; zero or negative for none.
    section: i16,
    kind: u16,
    class: u8,
    aux: u8,
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
    /// Zero-based index of the function's section.
    section: usize,
    /// File offset of the function's first byte.
    pub(crate) file: usize,
    /// Offset of the function's first byte within its section.
    pub(crate) start: usize,
    pub(crate) bytes: Vec<u8>,
    pub(crate) relocations: Vec<Relocation>,
    /// Offsets within the function that something outside its code names.
    pub(crate) barriers: BTreeSet<usize>,
    /// Offsets within the function that code or data outside it may enter,
    /// named by relocations from sections that are not debug records.
    pub(crate) entries: BTreeSet<usize>,
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
                    at,
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
                at,
                name: name(at)?,
                value: u32_at(bytes, at + 8)?,
                section: i16::from_le_bytes(u16_at(bytes, at + 12)?.to_le_bytes()),
                kind: u16_at(bytes, at + 14)?,
                class: bytes[at + 16],
                aux: bytes[at + 17],
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
            section: index,
            file: section.data + start,
            start,
            bytes,
            relocations,
            barriers: BTreeSet::new(),
            entries: BTreeSet::new(),
        };
        let inside = |offset: usize| (start < offset && offset < end).then(|| offset - start);
        // Line numbers: an address, or a symbol index where the line is 0.
        for line in (0..section.line_count).map(|item| section.lines + item * 6) {
            if u16_at(self.bytes, line + 4)? != 0 {
                function.barriers.extend(inside(u32_at(self.bytes, line)?));
            }
        }
        for (other, targets) in self.targets(index, start, end)? {
            let header = &self.sections[other];
            function
                .barriers
                .extend(targets.iter().filter_map(|&(_, target)| inside(target)));
            if !header.name.starts_with(".debug") {
                function
                    .entries
                    .extend(targets.iter().filter_map(|&(_, target)| inside(target)));
            }
            let data = self.data(header)?;
            for (field, width, target) in extents(&header.name, data, &targets)? {
                function
                    .barriers
                    .extend(inside(target + read(data, field, width)?));
            }
        }
        Ok(function)
    }

    fn data(&self, header: &Section) -> Result<&[u8]> {
        self.bytes
            .get(header.data..header.data + header.size)
            .context("a section lies outside the object")
    }

    /// For each section with relocations to places in `start..=end` of
    /// section `index`, each such relocation's offset and the place it names,
    /// by symbol plus in-place addend.
    fn targets(&self, index: usize, start: usize, end: usize) -> Result<Vec<(usize, Targets)>> {
        let mut found = Vec::new();
        for (other, header) in self.sections.iter().enumerate() {
            let mut targets = Vec::new();
            for (_, offset, symbol, kind) in self.relocations(other)? {
                let symbol = self.symbol(symbol)?;
                if usize::try_from(symbol.section).ok() != Some(index + 1) || kind == SECTION_NUMBER
                {
                    continue;
                }
                let target = symbol.value + u32_at(self.bytes, header.data + offset)?;
                if (start..=end).contains(&target) {
                    targets.push((offset, target));
                }
            }
            if !targets.is_empty() {
                found.push((other, targets));
            }
        }
        Ok(found)
    }

    /// The object with `function`'s code replaced by `code`. `place` maps
    /// each offset within the function where an instruction starts, and its
    /// end, to its new offset, and `fields` maps each relocated field's.
    /// Whatever names a place in or after the function moves with it:
    /// relocations, line numbers, symbols, section sizes and file pointers,
    /// relocation addends, `CodeView` 4 procedure and block extents, and
    /// FPO sizes.
    pub(crate) fn replace(
        &self,
        function: &Function,
        code: &[u8],
        place: &BTreeMap<usize, usize>,
        fields: &BTreeMap<usize, usize>,
    ) -> Result<Vec<u8>> {
        let cut = function.file + function.bytes.len();
        let mut new = Moved {
            function,
            length: code.len(),
            place,
            fields,
            bytes: [&self.bytes[..function.file], code, &self.bytes[cut..]].concat(),
        };
        new.update(8, 4, Moved::pointer, self.bytes)?;
        let own = &self.sections[function.section];
        for (index, section) in self.sections.iter().enumerate() {
            for field in [20, 24, 28] {
                new.update(section.at + field, 4, Moved::pointer, self.bytes)?;
            }
            if index == function.section {
                new.update(section.at + 16, 4, Moved::grow, self.bytes)?;
            }
        }
        for (entry, offset, ..) in self.relocations(function.section)? {
            let moved = size(new.relocated(offset)?)?;
            new.put(entry, 4, moved)?;
        }
        for line in (0..own.line_count).map(|item| own.lines + item * 6) {
            if u16_at(self.bytes, line + 4)? != 0 {
                new.update(line, 4, Moved::at, self.bytes)?;
            }
        }
        for symbol in self.symbols.iter().flatten() {
            if usize::try_from(symbol.section).ok() != Some(function.section + 1) {
                continue;
            }
            // `.lf` holds the function's count of lines, not a place.
            if symbol.value > function.start && !(symbol.class == 101 && symbol.name == ".lf") {
                new.update(symbol.at + 8, 4, Moved::at, self.bytes)?;
            }
            let aux = symbol.at + SYMBOL;
            if symbol.aux > 0 && symbol.class == 3 && symbol.value == 0 && symbol.name == own.name {
                new.update(aux, 4, Moved::grow, self.bytes)?;
            } else if symbol.aux > 0 && symbol.kind >> 4 == 2 && symbol.value == function.start {
                if u32_at(self.bytes, aux + 4)? != 0 {
                    new.update(aux + 4, 4, Moved::grow, self.bytes)?;
                }
                new.update(aux + 8, 4, Moved::pointer, self.bytes)?;
            }
        }
        self.readdress(&mut new)?;
        Ok(new.bytes)
    }

    /// Moves the in-place addends of relocations to places in the replaced
    /// function's section, and the extents debug records hold from them.
    fn readdress(&self, new: &mut Moved<'_>) -> Result<()> {
        let function = new.function;
        let (start, end) = (function.start, function.start + function.bytes.len());
        for (index, section) in self.sections.iter().enumerate() {
            for (_, offset, symbol, kind) in self.relocations(index)? {
                let symbol = self.symbol(symbol)?;
                if usize::try_from(symbol.section).ok() != Some(function.section + 1)
                    || !ADDRESSED.contains(&kind)
                {
                    continue;
                }
                let field = section.data + offset;
                let addend = i64::from(i32::from_le_bytes(
                    u32::try_from(u32_at(self.bytes, field)?)?.to_le_bytes(),
                ));
                let Ok(target) = usize::try_from(i64::try_from(symbol.value)? + addend) else {
                    continue;
                };
                if target < start {
                    continue;
                }
                let moved = i64::try_from(new.at(target)?)? - i64::try_from(new.at(symbol.value)?)?;
                new.put(
                    field,
                    4,
                    u32::from_le_bytes(i32::try_from(moved)?.to_le_bytes()),
                )?;
            }
        }
        for (other, targets) in self.targets(function.section, start, end)? {
            let header = &self.sections[other];
            let data = self.data(header)?;
            for (field, width, target) in extents(&header.name, data, &targets)? {
                let extent = new.at(target + read(data, field, width)?)? - new.at(target)?;
                new.put(header.data + field, width, size(extent)?)?;
            }
        }
        Ok(())
    }
}

fn size(value: usize) -> Result<u32> {
    u32::try_from(value).context("the object outgrows COFF")
}

/// An object being written with a function's code replaced, and where each
/// place in the old object lies in it.
struct Moved<'a> {
    function: &'a Function,
    /// The new code's length.
    length: usize,
    place: &'a BTreeMap<usize, usize>,
    fields: &'a BTreeMap<usize, usize>,
    bytes: Vec<u8>,
}

impl Moved<'_> {
    /// An offset after the function, moved by its change in length.
    fn grow(&self, value: usize) -> Result<usize> {
        (value + self.length)
            .checked_sub(self.function.bytes.len())
            .context("the function would shrink past an offset after it")
    }

    /// A file pointer, which moves when it lies after the function.
    fn pointer(&self, value: usize) -> Result<usize> {
        if value != 0 && value >= self.function.file + self.function.bytes.len() {
            self.grow(value)
        } else {
            Ok(value)
        }
    }

    /// A section offset as the new object has it.
    fn at(&self, offset: usize) -> Result<usize> {
        let (start, end) = (
            self.function.start,
            self.function.start + self.function.bytes.len(),
        );
        if offset < start {
            Ok(offset)
        } else if offset <= end {
            self.place
                .get(&(offset - start))
                .map(|&new| start + new)
                .with_context(|| {
                    format!(
                        "something names {:#x}, inside an instruction that moves",
                        offset - start
                    )
                })
        } else {
            self.grow(offset)
        }
    }

    /// The new section offset of a relocation at section offset `offset`,
    /// which moves with its field when it lies in the function.
    fn relocated(&self, offset: usize) -> Result<usize> {
        let start = self.function.start;
        if (start..start + self.function.bytes.len()).contains(&offset) {
            self.fields
                .get(&(offset - start))
                .map(|&new| start + new)
                .context("a relocated field has no new place")
        } else {
            self.at(offset)
        }
    }

    /// A file offset as the new object has it.
    fn file(&self, offset: usize) -> Result<usize> {
        let function = self.function;
        if offset < function.file {
            Ok(offset)
        } else if offset < function.file + function.bytes.len() {
            self.fields
                .get(&(offset - function.file))
                .map(|&new| function.file + new)
                .context("a relocated field has no new place")
        } else {
            self.grow(offset)
        }
    }

    /// Writes `value` to the `width`-byte field at old file offset `offset`.
    fn put(&mut self, offset: usize, width: usize, value: u32) -> Result<()> {
        let offset = self.file(offset)?;
        ensure!(
            width == 4 || value < 1 << (8 * width),
            "a debug record's field cannot hold the function's new extent"
        );
        self.bytes
            .get_mut(offset..offset + width)
            .context("the object is truncated")?
            .copy_from_slice(&value.to_le_bytes()[..width]);
        Ok(())
    }

    /// Rewrites the field at old file offset `offset` from its value in
    /// `old` through `change`.
    fn update(
        &mut self,
        offset: usize,
        width: usize,
        change: fn(&Self, usize) -> Result<usize>,
        old: &[u8],
    ) -> Result<()> {
        let value = size(change(self, read(old, offset, width)?)?)?;
        self.put(offset, width, value)
    }
}

/// The `width`-byte little-endian value at `at`.
fn read(data: &[u8], at: usize, width: usize) -> Result<usize> {
    match width {
        1 => Ok(usize::from(
            *data.get(at).context("the object is truncated")?,
        )),
        _ => u32_at(data, at),
    }
}

/// The fields of the debug records in section `name` that hold an offset
/// from a place `targets` names: each field's offset in `data`, its width,
/// and the place. `CodeView` 4 procedures hold their length and the offsets
/// where their debug range starts and ends, and blocks their length; FPO
/// data holds the procedure's size at +4 and its prologue's length at +14.
fn extents(
    name: &str,
    data: &[u8],
    targets: &[(usize, usize)],
) -> Result<Vec<(usize, usize, usize)>> {
    let mut extents = Vec::new();
    match name {
        ".debug$S" => {
            let mut at = match u32_at(data, 0)? {
                1 | 2 => 4,
                // A COMDAT function's own section holds records without a signature.
                first if first > 0xffff => 0,
                _ => bail!(
                    "the function's CodeView is not version 4, and only version 4 is kept in step"
                ),
            };
            while at + 4 <= data.len() {
                let next = at + 2 + usize::from(u16_at(data, at)?);
                let (kind, body) = (u16_at(data, at + 2)?, at + 4);
                if let Some(&(_, target)) = targets
                    .iter()
                    .find(|(offset, _)| (at..next).contains(offset))
                {
                    let fields: &[usize] = if PROCS.contains(&kind) {
                        &[12, 16, 20]
                    } else if BLOCKS.contains(&kind) {
                        &[8]
                    } else {
                        &[]
                    };
                    extents.extend(fields.iter().map(|field| (body + field, 4, target)));
                }
                at = next;
            }
        }
        ".debug$F" => {
            for &(offset, target) in targets {
                extents.extend([(offset + 4, 4, target), (offset + 14, 1, target)]);
            }
        }
        name if name.starts_with(".debug") => bail!(
            "{name} describes the function, and only CodeView 4 and FPO data are kept in step"
        ),
        _ => {}
    }
    Ok(extents)
}
