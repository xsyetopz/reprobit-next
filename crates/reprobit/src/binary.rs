//! Reading binaries: build-time metadata normalization, locating where two
//! images differ, and pulling one function out of an object file.

use anyhow::{Context, Result, bail};
use object::{Object, ObjectSection, ObjectSymbol};
use serde::Serialize;

/// The normalizations a target may name.
pub(crate) const NORMALIZATIONS: &[&str] = &["pe-timestamps"];

/// Applies the named normalizations to `bytes` and returns how many fields changed.
pub(crate) fn normalize(bytes: &mut [u8], names: &[String]) -> Result<usize> {
    let mut changed = 0;
    for name in names {
        changed += match name.as_str() {
            "pe-timestamps" => pe_timestamps(bytes)?,
            other => bail!(
                "unknown normalization {other}; known: {}",
                NORMALIZATIONS.join(", ")
            ),
        };
    }
    Ok(changed)
}

fn u16_at(bytes: &[u8], at: usize) -> Option<usize> {
    Some(usize::from(u16::from_le_bytes(
        bytes.get(at..at + 2)?.try_into().ok()?,
    )))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<usize> {
    usize::try_from(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?)).ok()
}

/// A PE section as `(name, rva, virtual size, file offset, file size)`.
type Section = (String, usize, usize, usize, usize);

/// A PE image's header offset and sections.
fn pe_sections(bytes: &[u8]) -> Option<(usize, Vec<Section>)> {
    let pe = u32_at(bytes, 0x3c)?;
    if bytes.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    let count = u16_at(bytes, pe + 6)?;
    let optional = u16_at(bytes, pe + 20)?;
    let table = pe + 24 + optional;
    let sections = (0..count)
        .filter_map(|index| {
            let at = table + index * 40;
            let name = String::from_utf8_lossy(bytes.get(at..at + 8)?)
                .trim_end_matches('\0')
                .to_owned();
            Some((
                name,
                u32_at(bytes, at + 12)?,
                u32_at(bytes, at + 8)?,
                u32_at(bytes, at + 20)?,
                u32_at(bytes, at + 16)?,
            ))
        })
        .collect();
    Some((pe, sections))
}

/// Zeroes the fields where a linker or resource compiler records when it ran:
/// the file header, export directory, debug directory entries, and resource
/// directory tables. Code and data are untouched.
fn pe_timestamps(bytes: &mut [u8]) -> Result<usize> {
    let (pe, sections) = pe_sections(bytes).context("pe-timestamps needs a PE image")?;
    let to_offset = |rva: usize| {
        sections
            .iter()
            .find(|(_, start, size, _, raw)| rva >= *start && rva < start + (*size).max(*raw))
            .map(|(_, start, _, offset, _)| rva - start + offset)
    };
    let optional = pe + 24;
    let directories = match u16_at(bytes, optional) {
        Some(0x20b) => optional + 112,
        _ => optional + 96,
    };
    let directory = |index: usize| {
        Some((
            u32_at(bytes, directories + index * 8)?,
            u32_at(bytes, directories + index * 8 + 4)?,
        ))
    };
    let mut fields = vec![pe + 8];
    if let Some((rva, _)) = directory(0).filter(|(rva, size)| *rva != 0 && *size != 0)
        && let Some(at) = to_offset(rva)
    {
        fields.push(at + 4);
    }
    if let Some((rva, size)) = directory(6).filter(|(rva, _)| *rva != 0)
        && let Some(at) = to_offset(rva)
    {
        fields.extend((0..size / 28).map(|entry| at + entry * 28 + 4));
    }
    if let Some((rva, _)) = directory(2).filter(|(rva, _)| *rva != 0)
        && let Some(base) = to_offset(rva)
    {
        let mut pending = vec![base];
        while let Some(table) = pending.pop() {
            fields.push(table + 4);
            let entries =
                u16_at(bytes, table + 12).unwrap_or(0) + u16_at(bytes, table + 14).unwrap_or(0);
            for entry in 0..entries {
                let target = u32_at(bytes, table + 16 + entry * 8 + 4).unwrap_or(0);
                if target & 0x8000_0000 != 0 && pending.len() < 4096 {
                    pending.push(base + (target & 0x7fff_ffff));
                }
            }
        }
    }
    let mut changed = 0;
    for at in fields {
        if let Some(field) = bytes.get_mut(at..at + 4)
            && field.iter().any(|byte| *byte != 0)
        {
            field.fill(0);
            changed += 1;
        }
    }
    Ok(changed)
}

/// One run of differing bytes.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Difference {
    /// File offset of the first differing byte.
    pub(crate) offset: usize,
    /// Length of the run.
    pub(crate) length: usize,
    /// The PE section holding it, when known.
    pub(crate) section: Option<String>,
    /// Its relative virtual address, when known.
    pub(crate) rva: Option<usize>,
}

/// Up to `limit` runs where `built` and `reference` differ.
pub(crate) fn differences(built: &[u8], reference: &[u8], limit: usize) -> Vec<Difference> {
    let sections = pe_sections(reference).map_or_default(|(_, sections)| sections);
    let mut runs: Vec<Difference> = Vec::new();
    let length = built.len().max(reference.len());
    let mut offset = 0;
    while offset < length && runs.len() < limit {
        if built.get(offset) == reference.get(offset) {
            offset += 1;
            continue;
        }
        let start = offset;
        while offset < length && built.get(offset) != reference.get(offset) {
            offset += 1;
        }
        let place = sections
            .iter()
            .find(|(_, _, _, raw, size)| start >= *raw && start < raw + size);
        runs.push(Difference {
            offset: start,
            length: offset - start,
            section: place.map(|(name, ..)| name.clone()),
            rva: place.map(|(_, rva, _, raw, _)| start - raw + rva),
        });
    }
    runs
}

/// A function's bytes and relocations, comparable across two object files.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Function {
    pub(crate) bytes: Vec<u8>,
    /// `(offset within the function, target symbol name)`.
    pub(crate) relocations: Vec<(u64, String)>,
}

/// Extracts `symbol` from a COFF, ELF, or Mach-O object. Its extent runs to
/// the next symbol in the same section, or the section's end.
pub(crate) fn function(object_bytes: &[u8], symbol: &str) -> Result<Function> {
    let file = object::File::parse(object_bytes)?;
    let found = file
        .symbols()
        .find(|item| item.name() == Ok(symbol))
        .with_context(|| format!("the object has no symbol {symbol}"))?;
    let index = found
        .section_index()
        .context("the symbol is not defined in a section")?;
    let section = file.section_by_index(index)?;
    let start = found.address() - section.address();
    let end = if found.size() > 0 {
        start + found.size()
    } else {
        file.symbols()
            .filter(|item| {
                item.section_index() == Some(index) && item.address() - section.address() > start
            })
            .map(|item| item.address() - section.address())
            .min()
            .unwrap_or(section.size())
    };
    let data = section.data()?;
    let bytes = data
        .get(usize::try_from(start)?..usize::try_from(end)?)
        .context("the symbol lies outside its section")?
        .to_vec();
    let mut relocations: Vec<(u64, String)> = section
        .relocations()
        .filter(|(offset, _)| (start..end).contains(offset))
        .map(|(offset, relocation)| {
            let target = match relocation.target() {
                object::RelocationTarget::Symbol(target) => file
                    .symbol_by_index(target)
                    .ok()
                    .and_then(|item| item.name().ok().map(str::to_owned)),
                _ => None,
            };
            (offset - start, target.unwrap_or_default())
        })
        .collect();
    relocations.sort();
    Ok(Function { bytes, relocations })
}
