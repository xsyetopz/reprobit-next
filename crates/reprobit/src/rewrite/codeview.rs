//! `CodeView` debug records in a COFF object's `.debug$S`, renamed to follow
//! a function's code: register variables and register-relative locals in
//! the function's symbol scope, and the frame programs that say where its
//! saved registers live.

use std::collections::BTreeMap;

use anyhow::{Result, bail, ensure};
use object::{Object, ObjectSection};

use super::{Anchor, Registers, coff_grow};

/// Records that open the function's scope; it starts at the one whose
/// address relocation points at the function.
const PROCS: [u16; 8] = [
    0x0204, 0x0205, 0x100a, 0x100b, 0x110f, 0x1110, 0x1146, 0x1147,
];
/// Records that open a nested scope: thunks, blocks, `with`, inline sites.
const OPENS: [u16; 8] = [
    0x0206, 0x0207, 0x0208, 0x1102, 0x1103, 0x1104, 0x114d, 0x115d,
];
/// `S_END`, `S_INLINESITE_END`, and `S_PROC_ID_END`.
const CLOSES: [u16; 3] = [0x0006, 0x114e, 0x114f];

/// Renames registers named by `symbol`'s records in every `.debug$S`.
pub(super) fn rename(
    mut bytes: Vec<u8>,
    anchor: &Anchor,
    registers: &Registers,
) -> Result<Vec<u8>> {
    // CodeView numbers each register's 32- and 16-bit forms; only `ebx`
    // has byte forms (`bl` 4, `bh` 8), which no other register can take.
    let number = |name: &str| match name {
        "ebx" => (20, 12),
        "esi" => (23, 15),
        _ => (24, 16),
    };
    let mut numbers = BTreeMap::new();
    for (from, to) in registers {
        let (from, to) = (number(from), number(to));
        numbers.insert(from.0, to.0);
        numbers.insert(from.1, to.1);
    }
    let renames_ebx = registers.contains_key("ebx");
    let names: BTreeMap<String, String> = registers
        .iter()
        .map(|(from, to)| (format!("${from}"), format!("${to}")))
        .collect();
    let sections: Vec<_> = {
        let file = object::File::parse(&*bytes)?;
        file.sections()
            .filter(|section| section.name() == Ok(".debug$S"))
            .map(|section| -> Result<_> {
                let anchors = anchor.hits(&file, &section)?;
                Ok((section.index(), anchors))
            })
            .collect::<Result<_>>()?
    };
    for (index, anchors) in sections {
        let (start, data) = {
            let file = object::File::parse(&*bytes)?;
            let section = file.section_by_index(index)?;
            let Some((start, _)) = section.file_range() else {
                continue;
            };
            (usize::try_from(start)?, section.data()?.to_vec())
        };
        let mut scan = Scan {
            data,
            anchors,
            numbers: &numbers,
            renames_ebx,
            frames: Vec::new(),
            strings: None,
        };
        scan.section()?;
        let grown = scan.frames(&names)?;
        match grown {
            None => bytes[start..start + scan.data.len()].copy_from_slice(&scan.data),
            Some((at, by)) => bytes = coff_grow(bytes, index, scan.data, at, by)?,
        }
    }
    Ok(bytes)
}

struct Scan<'a> {
    data: Vec<u8>,
    anchors: Vec<usize>,
    numbers: &'a BTreeMap<u16, u16>,
    renames_ebx: bool,
    /// Offsets of the frame-program string fields that belong to the function.
    frames: Vec<usize>,
    /// The string table subsection's content range.
    strings: Option<(usize, usize)>,
}

impl Scan<'_> {
    fn u16(&self, at: usize) -> Result<u16> {
        match self.data.get(at..at + 2) {
            Some(&[low, high]) => Ok(u16::from_le_bytes([low, high])),
            _ => bail!("a CodeView record runs past its section"),
        }
    }

    fn u32(&self, at: usize) -> Result<usize> {
        Ok(usize::from(self.u16(at)?) | usize::from(self.u16(at + 2)?) << 16)
    }

    fn section(&mut self) -> Result<()> {
        if self.data.len() < 4 {
            return Ok(());
        }
        match self.u32(0)? {
            1 | 2 => self.records(4, self.data.len()),
            4 => {
                let mut at = 4;
                while at + 8 <= self.data.len() {
                    let (kind, length) = (self.u32(at)?, self.u32(at + 4)?);
                    let body = at + 8;
                    ensure!(
                        body + length <= self.data.len(),
                        "a CodeView subsection runs past its section"
                    );
                    match kind {
                        0xf1 => self.records(body, body + length)?,
                        0xf3 => self.strings = Some((body, body + length)),
                        // Frame data: a relocated function address, then 32-byte entries.
                        0xf5 if self.anchors.contains(&body) => self.frames.extend(
                            (body + 4..body + length)
                                .step_by(32)
                                .map(|entry| entry + 20),
                        ),
                        _ => {}
                    }
                    at = body + length.next_multiple_of(4);
                }
                Ok(())
            }
            // A COMDAT function's own section holds records without a signature.
            first if first > 0xffff => self.records(0, self.data.len()),
            signature => bail!("unknown CodeView signature {signature}"),
        }
    }

    fn records(&mut self, start: usize, end: usize) -> Result<()> {
        let mut depth = 0_usize;
        let mut at = start;
        while at + 4 <= end {
            let next = at + 2 + usize::from(self.u16(at)?);
            let kind = self.u16(at + 2)?;
            let body = at + 4;
            ensure!(next <= end, "a CodeView record runs past its subsection");
            if depth == 0 {
                if PROCS.contains(&kind)
                    && self
                        .anchors
                        .iter()
                        .any(|anchor| (at..next).contains(anchor))
                {
                    depth = 1;
                }
            } else if PROCS.contains(&kind) || OPENS.contains(&kind) {
                depth += 1;
            } else if CLOSES.contains(&kind) {
                depth -= 1;
            } else {
                self.record(kind, body, next)?;
            }
            at = next;
        }
        Ok(())
    }

    /// Renames the registers one record in the function's scope names.
    fn record(&mut self, kind: u16, body: usize, end: usize) -> Result<()> {
        match kind {
            // S_REGISTER_16t
            0x0002 => self.register(body + 2, 2),
            // S_REGREL32_16t
            0x020c => self.register(body + 4, 2),
            // S_REGISTER_ST, S_REGISTER
            0x1001 | 0x1106 => self.register(body + 4, 2),
            // S_REGREL32_ST, S_REGREL32, S_REGREL32_INDIR
            0x100d | 0x1111 | 0x1171 => self.register(body + 8, 2),
            // S_DEFRANGE_REGISTER, _SUBFIELD_REGISTER, _REGISTER_REL, _REGISTER_REL_INDIR
            0x1141 | 0x1143 | 0x1145 | 0x1177 => self.register(body, 2),
            // S_MANYREG_16t: a byte count, then one byte per register.
            0x000c => (body + 3..end)
                .take(usize::from(self.data[body + 2]))
                .try_for_each(|at| self.register(at, 1)),
            // S_MANYREG_ST, S_MANYREG
            0x1005 | 0x110a => (body + 5..end)
                .take(usize::from(self.data[body + 4]))
                .try_for_each(|at| self.register(at, 1)),
            // S_MANYREG2_ST, S_MANYREG2: a 16-bit count of 16-bit registers.
            0x1014 | 0x1117 => (0..usize::from(self.u16(body + 4)?))
                .try_for_each(|index| self.register(body + 6 + 2 * index, 2)),
            // S_FRAMEPROC: locals or parameters addressed from `ebx`.
            0x1012 => {
                let flags = self.u32(body + 22)?;
                ensure!(
                    !self.renames_ebx || (flags >> 14) & 3 != 3 && (flags >> 16) & 3 != 3,
                    "the function addresses its frame from ebx, which CodeView cannot record under another register"
                );
                Ok(())
            }
            // S_ENTRYTHIS
            0x000e => bail!(
                "the function's debug records hold an S_ENTRYTHIS record, which is not renamed"
            ),
            _ => Ok(()),
        }
    }

    fn register(&mut self, at: usize, width: usize) -> Result<()> {
        let number = if width == 1 {
            u16::from(
                *self
                    .data
                    .get(at)
                    .ok_or_else(|| anyhow::anyhow!("a CodeView record runs past its section"))?,
            )
        } else {
            self.u16(at)?
        };
        ensure!(
            !self.renames_ebx || !matches!(number, 4 | 8),
            "the function's debug records name bl or bh, which no renamed register has"
        );
        if let Some(&to) = self.numbers.get(&number) {
            let to = to.to_le_bytes();
            self.data[at..at + width].copy_from_slice(&to[..width]);
        }
        Ok(())
    }

    /// Points each of the function's frame programs at its renamed text,
    /// reusing an equal string or appending one. Other functions may share
    /// the original string, so it is never edited in place. Returns where
    /// the section grew and by how much.
    fn frames(&mut self, names: &BTreeMap<String, String>) -> Result<Option<(usize, usize)>> {
        if self.frames.is_empty() {
            return Ok(None);
        }
        let (start, end) = self
            .strings
            .ok_or_else(|| anyhow::anyhow!("frame data without a string table"))?;
        let table = self.data[start..end].to_vec();
        let text = |offset: usize| -> Result<&[u8]> {
            let rest = table
                .get(offset..)
                .ok_or_else(|| anyhow::anyhow!("a frame program lies outside the string table"))?;
            Ok(&rest[..rest
                .iter()
                .position(|&byte| byte == 0)
                .unwrap_or(rest.len())])
        };
        let mut appended: Vec<u8> = Vec::new();
        let mut fields = Vec::new();
        for &field in &self.frames {
            let old = text(self.u32(field)?)?;
            let renamed: Vec<u8> = String::from_utf8_lossy(old)
                .split(' ')
                .map(|token| names.get(token).map_or(token, String::as_str))
                .collect::<Vec<_>>()
                .join(" ")
                .into_bytes();
            let mut wanted = renamed.clone();
            wanted.push(0);
            let existing = table
                .windows(wanted.len())
                .enumerate()
                .find(|(offset, window)| {
                    *window == wanted && (*offset == 0 || table[offset - 1] == 0)
                })
                .map(|(offset, _)| offset);
            let offset = existing.unwrap_or_else(|| {
                let at = appended
                    .windows(wanted.len())
                    .position(|window| window == wanted)
                    .unwrap_or_else(|| {
                        appended.extend(&wanted);
                        appended.len() - wanted.len()
                    });
                table.len() + at
            });
            fields.push((field, offset));
        }
        for (field, offset) in fields {
            let offset = u32::try_from(offset)?.to_le_bytes();
            self.data[field..field + 4].copy_from_slice(&offset);
        }
        if appended.is_empty() {
            return Ok(None);
        }
        // Grow by whole words so every later subsection keeps its alignment.
        appended.resize(appended.len().next_multiple_of(4), 0);
        let by = appended.len();
        let length = u32::try_from(end - start + by)?.to_le_bytes();
        self.data[start - 4..start].copy_from_slice(&length);
        // The table's own padding moves along with what follows it.
        self.data.splice(end..end, appended);
        Ok(Some((end, by)))
    }
}
