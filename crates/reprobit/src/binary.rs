//! Reading binaries: build-time metadata normalization, locating where two
//! images differ, and pulling one function out of an object file.

use anyhow::{Context, Result, bail, ensure};
use object::{Object, ObjectSection, ObjectSymbol};
use serde::Serialize;

/// The normalizations a target may name.
pub(crate) const NORMALIZATIONS: &[&str] = &["pe-timestamps", "pe-checksum", "build-id"];

/// The fields a target may stamp.
pub(crate) const STAMPS: &[&str] = &["pe-timestamp", "pe-checksum"];

/// Writes each declared field value into `bytes`.
pub(crate) fn stamp(
    bytes: &mut [u8],
    fields: &std::collections::BTreeMap<String, u32>,
) -> Result<()> {
    for (name, value) in fields {
        let (pe, _) = pe_sections(bytes).with_context(|| format!("{name} needs a PE image"))?;
        let at = match name.as_str() {
            "pe-timestamp" => pe + 8,
            "pe-checksum" => pe + 24 + 64,
            other => bail!("unknown stamp {other}; known: {}", STAMPS.join(", ")),
        };
        bytes
            .get_mut(at..at + 4)
            .context("the PE header is truncated")?
            .copy_from_slice(&value.to_le_bytes());
    }
    Ok(())
}

/// Applies the named normalizations to `bytes` and returns how many fields changed.
pub(crate) fn normalize(bytes: &mut [u8], names: &[String]) -> Result<usize> {
    let mut changed = 0;
    for name in names {
        changed += match name.as_str() {
            "pe-timestamps" => pe_timestamps(bytes)?,
            "pe-checksum" => {
                let (pe, _) = pe_sections(bytes).context("pe-checksum needs a PE image")?;
                zero(bytes, &[(pe + 24 + 64, 4)])
            }
            "build-id" => {
                let fields = build_id(bytes);
                ensure!(
                    !fields.is_empty(),
                    "build-id found no PE debug signature, ELF build-id, or Mach-O UUID"
                );
                zero(bytes, &fields)
            }
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

/// The file offset and size of PE data directory `index`, when present.
fn pe_directory(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    let (pe, sections) = pe_sections(bytes)?;
    let optional = pe + 24;
    let directories = match u16_at(bytes, optional) {
        Some(0x20b) => optional + 112,
        _ => optional + 96,
    };
    let rva = u32_at(bytes, directories + index * 8)?;
    let size = u32_at(bytes, directories + index * 8 + 4)?;
    let offset = sections
        .iter()
        .find(|(_, start, virtual_size, _, raw)| {
            rva != 0 && rva >= *start && rva < start + (*virtual_size).max(*raw)
        })
        .map(|(_, start, _, offset, _)| rva - start + offset)?;
    Some((offset, size))
}

/// Zeroes the fields where a linker or resource compiler records when it ran:
/// the file header, export directory, debug directory entries, and resource
/// directory tables. Code and data are untouched.
fn pe_timestamps(bytes: &mut [u8]) -> Result<usize> {
    let (pe, _) = pe_sections(bytes).context("pe-timestamps needs a PE image")?;
    let mut fields = vec![(pe + 8, 4)];
    if let Some((at, _)) = pe_directory(bytes, 0).filter(|(_, size)| *size != 0) {
        fields.push((at + 4, 4));
    }
    if let Some((at, size)) = pe_directory(bytes, 6) {
        fields.extend((0..size / 28).map(|entry| (at + entry * 28 + 4, 4)));
    }
    if let Some((base, _)) = pe_directory(bytes, 2) {
        let mut pending = vec![base];
        while let Some(table) = pending.pop() {
            fields.push((table + 4, 4));
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
    Ok(zero(bytes, &fields))
}

/// Zeroes each `(offset, length)` field and returns how many were not zero.
fn zero(bytes: &mut [u8], fields: &[(usize, usize)]) -> usize {
    let mut changed = 0;
    for (at, length) in fields {
        if let Some(field) = bytes.get_mut(*at..at + length)
            && field.iter().any(|byte| *byte != 0)
        {
            field.fill(0);
            changed += 1;
        }
    }
    changed
}

/// The fields that identify one link rather than its content: the PE
/// `CodeView` signature (`RSDS` GUID and age, or `NB10` signature and age),
/// the ELF GNU build-id, and each Mach-O `LC_UUID` with the code signature
/// that hashes it, universal binaries included. Linkers fill them with
/// random or time-based values.
fn build_id(bytes: &[u8]) -> Vec<(usize, usize)> {
    if pe_sections(bytes).is_some() {
        let Some((table, size)) = pe_directory(bytes, 6) else {
            return Vec::new();
        };
        return (0..size / 28)
            .filter_map(|entry| {
                let at = table + entry * 28;
                (u32_at(bytes, at + 12)? == 2).then_some(())?;
                let data = u32_at(bytes, at + 24)?;
                match bytes.get(data..data + 4)? {
                    b"RSDS" => Some((data + 4, 20)),
                    b"NB10" => Some((data + 8, 8)),
                    _ => None,
                }
            })
            .collect();
    }
    if let Some(fields) = mach_uuids(bytes, 0) {
        return fields;
    }
    // The build-id is a slice of `bytes`, so its address gives its offset.
    object::File::parse(bytes)
        .ok()
        .and_then(|file| file.build_id().ok().flatten())
        .map_or_default(|id| vec![(id.as_ptr().addr() - bytes.as_ptr().addr(), id.len())])
}

/// Each `LC_UUID` payload and code signature in a thin or universal Mach-O
/// at `base`, of either byte order, since `PowerPC` images are big-endian.
fn mach_uuids(bytes: &[u8], base: usize) -> Option<Vec<(usize, usize)>> {
    let word = |at: usize, big: bool| {
        let raw: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
        usize::try_from(if big {
            u32::from_be_bytes(raw)
        } else {
            u32::from_le_bytes(raw)
        })
        .ok()
    };
    match word(base, true)? {
        0xcafe_babe => {
            let mut fields = Vec::new();
            for arch in 0..word(base + 4, true)?.min(64) {
                fields.extend(mach_uuids(bytes, word(base + 8 + arch * 20 + 8, true)?)?);
            }
            Some(fields)
        }
        magic @ (0xfeed_face | 0xfeed_facf | 0xcefa_edfe | 0xcffa_edfe) => {
            let big = matches!(magic, 0xfeed_face | 0xfeed_facf);
            let wide = matches!(magic, 0xfeed_facf | 0xcffa_edfe);
            let mut at = base + if wide { 32 } else { 28 };
            let mut fields = Vec::new();
            for _ in 0..word(base + 16, big)? {
                match word(at, big)? {
                    0x1b => fields.push((at + 8, 16)),
                    // The code signature hashes the pages that hold the UUID.
                    0x1d => fields.push((base + word(at + 8, big)?, word(at + 12, big)?)),
                    _ => {}
                }
                at += word(at + 4, big)?.max(8);
            }
            Some(fields)
        }
        _ => None,
    }
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
    locate(object_bytes, symbol).map(|(_, function)| function)
}

/// Like [`function`], with the file offset of the function's first byte.
pub(crate) fn locate(object_bytes: &[u8], symbol: &str) -> Result<(usize, Function)> {
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
    let (file_start, _) = section
        .file_range()
        .context("the symbol's section has no bytes in the file")?;
    Ok((
        usize::try_from(file_start + start)?,
        Function { bytes, relocations },
    ))
}
