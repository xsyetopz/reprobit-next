//! DWARF in an i386 object, renamed to follow a function's code: location
//! expressions in the function's debugging entries, and its call-frame
//! instructions in `.eh_frame` and `.debug_frame`, which unwinders use to
//! restore the registers it saved.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail, ensure};
use gimli::{
    AttributeValue, DW_AT_MIPS_linkage_name, DW_AT_abstract_origin, DW_AT_high_pc,
    DW_AT_linkage_name, DW_AT_low_pc, DW_AT_name, DW_AT_ranges, DW_AT_specification,
    DW_TAG_subprogram, EndianSlice, Expression, LittleEndian, Operation, RawLocListEntry,
};
use object::{CompressionFormat, Object, ObjectSection};

use super::{Anchor, Registers};

type Slice<'a> = EndianSlice<'a, LittleEndian>;

/// Renames DWARF register numbers (`ebx` 3, `esi` 6, `edi` 7) in place.
pub(super) fn rename(
    bytes: &mut [u8],
    symbol: &str,
    anchor: &Anchor,
    registers: &Registers,
) -> Result<()> {
    let number = |name: &str| match name {
        "ebx" => 3,
        "esi" => 6,
        _ => 7,
    };
    let map: BTreeMap<u16, u8> = registers
        .iter()
        .map(|(from, to)| (number(from), u8::try_from(number(to)).unwrap_or_default()))
        .collect();
    let patches = {
        let file = object::File::parse(&*bytes)?;
        let mut walk = Walk {
            base: bytes.as_ptr().addr(),
            map: &map,
            patches: Vec::new(),
        };
        for section in file.sections() {
            let name = section.name()?;
            if name.starts_with(".debug") || name == ".eh_frame" {
                ensure!(
                    section.compressed_file_range()?.format == CompressionFormat::None,
                    "{name} is compressed"
                );
            }
            if name == ".eh_frame" || name == ".debug_frame" {
                walk.frames(
                    section.data()?,
                    name == ".eh_frame",
                    &anchor.hits(&file, &section)?,
                )?;
            }
        }
        walk.entries(&file, symbol)?;
        walk.patches
    };
    for (at, value) in patches {
        bytes[at] = value;
    }
    Ok(())
}

struct Walk<'a> {
    base: usize,
    map: &'a BTreeMap<u16, u8>,
    /// File offsets and their renamed bytes.
    patches: Vec<(usize, u8)>,
}

impl Walk<'_> {
    fn at(&self, byte: &u8) -> usize {
        std::ptr::from_ref(byte).addr() - self.base
    }

    /// Renames every location in the function's debugging entries.
    fn entries(&mut self, file: &object::File<'_>, symbol: &str) -> Result<()> {
        let dwarf = gimli::Dwarf::load(|id| -> Result<Slice<'_>> {
            let data = match file.section_by_name(id.name()) {
                Some(section) => section.data()?,
                None => &[],
            };
            Ok(EndianSlice::new(data, LittleEndian))
        })?;
        let mut found = 0;
        let mut units = dwarf.units();
        while let Some(header) = units.next()? {
            let unit = dwarf.unit(header)?;
            let named = |entry: &gimli::DebuggingInformationEntry<Slice<'_>>| -> Result<bool> {
                for name in [DW_AT_linkage_name, DW_AT_MIPS_linkage_name, DW_AT_name] {
                    if let Some(value) = entry.attr_value(name) {
                        let text = dwarf.attr_string(&unit, value)?;
                        let text = text.slice();
                        // COFF prefixes C names with `_`.
                        if text == symbol.as_bytes()
                            || symbol
                                .strip_prefix('_')
                                .is_some_and(|bare| bare.as_bytes() == text)
                        {
                            return Ok(true);
                        }
                    }
                }
                Ok(false)
            };
            let mut inside: Option<isize> = None;
            let mut entries = unit.entries();
            while let Some(entry) = entries.next_dfs()? {
                if inside.is_some_and(|depth| entry.depth() <= depth) {
                    inside = None;
                }
                if inside.is_none()
                    && entry.tag() == DW_TAG_subprogram
                    && (entry.has_attr(DW_AT_low_pc)
                        || entry.has_attr(DW_AT_ranges)
                        || entry.has_attr(DW_AT_high_pc))
                {
                    let mut matches = named(entry)?;
                    for link in [DW_AT_specification, DW_AT_abstract_origin] {
                        if let Some(AttributeValue::UnitRef(offset)) = entry.attr_value(link) {
                            matches |= named(&unit.entry(offset)?)?;
                        }
                    }
                    if matches {
                        found += 1;
                        inside = Some(entry.depth());
                    }
                }
                if inside.is_none() {
                    continue;
                }
                for attribute in entry.attrs() {
                    let value = attribute.value();
                    if let AttributeValue::Exprloc(expression) = value {
                        self.expression(expression, unit.encoding(), false)?;
                    } else if let Some(offset) = dwarf.attr_locations_offset(&unit, value)? {
                        let mut list = dwarf.raw_locations(&unit, offset)?;
                        while let Some(entry) = list.next()? {
                            if let RawLocListEntry::AddressOrOffsetPair { data, .. }
                            | RawLocListEntry::StartxEndx { data, .. }
                            | RawLocListEntry::StartxLength { data, .. }
                            | RawLocListEntry::OffsetPair { data, .. }
                            | RawLocListEntry::DefaultLocation { data }
                            | RawLocListEntry::StartEnd { data, .. }
                            | RawLocListEntry::StartLength { data, .. } = entry
                            {
                                self.expression(data, unit.encoding(), false)?;
                            }
                        }
                    }
                }
            }
        }
        ensure!(found <= 1, "{found} debugging entries describe {symbol}");
        Ok(())
    }

    /// Renames the registers one DWARF expression names. Inside an entry
    /// value, a renamed register means the caller's, which the rename does
    /// not touch, so it is refused rather than guessed.
    fn expression(
        &mut self,
        expression: Expression<Slice<'_>>,
        encoding: gimli::Encoding,
        nested: bool,
    ) -> Result<()> {
        let mut rest = expression.0;
        while let Some(opcode) = rest.slice().first() {
            let (at, opcode, operand) = (self.at(opcode), *opcode, rest.slice().get(1).copied());
            match Operation::parse(&mut rest, encoding)? {
                Operation::Register { register } | Operation::RegisterOffset { register, .. } => {
                    if let Some(&to) = self.map.get(&register.0) {
                        ensure!(!nested, "an entry value names a renamed register");
                        if (0x50..=0x8f).contains(&opcode) {
                            let base = if opcode < 0x70 { 0x50 } else { 0x70 };
                            self.patches.push((at, base + to));
                        } else {
                            ensure!(
                                operand == u8::try_from(register.0).ok(),
                                "a register operand is not in its shortest form"
                            );
                            self.patches.push((at + 1, to));
                        }
                    }
                }
                Operation::EntryValue { expression } => {
                    self.expression(Expression(expression), encoding, true)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Renames the call-frame instructions of the function's frame
    /// description entries. A CIE is shared by many functions, so one whose
    /// instructions name a renamed register is refused.
    fn frames(&mut self, data: &[u8], eh: bool, anchors: &[usize]) -> Result<()> {
        let word = |at: usize| -> Result<usize> {
            let field = data
                .get(at..at + 4)
                .context("call-frame information runs past its section")?;
            Ok(usize::try_from(u32::from_le_bytes([
                field[0], field[1], field[2], field[3],
            ]))?)
        };
        let mut cies = BTreeMap::new();
        let mut at = 0;
        while at + 4 <= data.len() {
            let length = word(at)?;
            ensure!(length != 0xffff_ffff, "64-bit call-frame information");
            let (body, end) = (at + 4, at + 4 + length);
            ensure!(
                end <= data.len(),
                "call-frame information runs past its section"
            );
            if length == 0 {
                at = end;
                continue;
            }
            let id = word(body)?;
            if (eh && id == 0) || (!eh && id == 0xffff_ffff) {
                let cie = Cie::parse(data, body + 4, eh)?;
                let before = self.patches.len();
                self.instructions(data, cie.instructions, end, cie.pointer)?;
                ensure!(
                    self.patches.len() == before,
                    "a shared CIE names a renamed register"
                );
                cies.insert(at, cie);
            } else {
                let cie = cies
                    .get(&if eh { body - id } else { id })
                    .context("a frame description entry precedes its CIE")?;
                let pc = body + 4;
                if anchors.contains(&pc) {
                    let mut cursor = pc + 2 * cie.pointer;
                    if cie.augmented {
                        let (skip, size) = leb(data, cursor)?;
                        cursor += size + usize::try_from(skip)?;
                    }
                    self.instructions(data, cursor, end, cie.pointer)?;
                }
            }
            at = end;
        }
        Ok(())
    }

    fn instructions(
        &mut self,
        data: &[u8],
        mut at: usize,
        end: usize,
        pointer: usize,
    ) -> Result<()> {
        let encoding = gimli::Encoding {
            format: gimli::Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        while at < end {
            let opcode = data[at];
            at += 1;
            let register = |walk: &mut Self, at: usize| {
                if let Some(&to) = walk.map.get(&u16::from(data[at])) {
                    walk.patches.push((walk.at(&data[at]), to));
                }
            };
            let skip = |at: &mut usize| -> Result<()> {
                *at += leb(data, *at)?.1;
                Ok(())
            };
            match opcode {
                0x40..=0x7f | 0x00 | 0x0a | 0x0b => {}
                // DW_CFA_offset and DW_CFA_restore hold the register in the opcode.
                0x80..=0xff => {
                    if let Some(&to) = self.map.get(&u16::from(opcode & 0x3f)) {
                        self.patches
                            .push((self.at(&data[at - 1]), (opcode & 0xc0) | to));
                    }
                    if opcode < 0xc0 {
                        skip(&mut at)?;
                    }
                }
                0x01 => at += pointer,
                0x02 => at += 1,
                0x03 => at += 2,
                0x04 => at += 4,
                0x0e | 0x13 | 0x2e => skip(&mut at)?,
                0x06..=0x08 | 0x0d => {
                    register(self, at);
                    skip(&mut at)?;
                }
                0x05 | 0x0c | 0x11 | 0x12 | 0x14 | 0x15 | 0x2f => {
                    register(self, at);
                    skip(&mut at)?;
                    skip(&mut at)?;
                }
                0x09 => {
                    register(self, at);
                    skip(&mut at)?;
                    register(self, at);
                    skip(&mut at)?;
                }
                0x0f | 0x10 | 0x16 => {
                    if opcode != 0x0f {
                        register(self, at);
                        skip(&mut at)?;
                    }
                    let (length, size) = leb(data, at)?;
                    let block = data
                        .get(at + size..at + size + usize::try_from(length)?)
                        .context("a call-frame expression runs past its entry")?;
                    self.expression(
                        Expression(EndianSlice::new(block, LittleEndian)),
                        encoding,
                        false,
                    )?;
                    at += size + block.len();
                }
                _ => bail!("unknown call-frame instruction {opcode:#04x}"),
            }
        }
        Ok(())
    }
}

/// What a frame description entry needs from its CIE.
struct Cie {
    /// Where the CIE's initial instructions start.
    instructions: usize,
    /// The size of an encoded address.
    pointer: usize,
    /// Whether entries carry augmentation data.
    augmented: bool,
}

impl Cie {
    fn parse(data: &[u8], mut at: usize, eh: bool) -> Result<Self> {
        let version = *data.get(at).context("truncated CIE")?;
        at += 1;
        let text_end = data[at..]
            .iter()
            .position(|&byte| byte == 0)
            .context("truncated CIE")?;
        let augmentation = data[at..at + text_end].to_vec();
        at += text_end + 1;
        if augmentation.windows(2).any(|pair| pair == b"eh") {
            at += 4;
        }
        if !eh && version >= 4 {
            at += 2;
        }
        at += leb(data, at)?.1 + leb(data, at + leb(data, at)?.1)?.1;
        at += if version == 1 { 1 } else { leb(data, at)?.1 };
        let mut pointer = 4;
        let augmented = augmentation.first() == Some(&b'z');
        if augmented {
            let (length, size) = leb(data, at)?;
            let mut cursor = at + size;
            for letter in &augmentation[1..] {
                match letter {
                    b'R' => pointer = encoded_size(data[cursor])?,
                    b'P' => cursor += encoded_size(data[cursor])?,
                    b'L' | b'S' | b'B' => {}
                    other => bail!("unknown CIE augmentation {}", char::from(*other)),
                }
                if *letter != b'S' && *letter != b'B' {
                    cursor += 1;
                }
            }
            at += size + usize::try_from(length)?;
        }
        Ok(Self {
            instructions: at,
            pointer,
            augmented,
        })
    }
}

/// The size of a pointer in a `DW_EH_PE` encoding on a 32-bit target.
fn encoded_size(encoding: u8) -> Result<usize> {
    Ok(match encoding & 0x0f {
        0x00 | 0x03 | 0x0b => 4,
        0x02 | 0x0a => 2,
        0x04 | 0x0c => 8,
        _ => bail!("unsupported pointer encoding {encoding:#04x}"),
    })
}

/// A LEB128 number's value, as unsigned bits, and its length in bytes.
fn leb(data: &[u8], at: usize) -> Result<(u64, usize)> {
    let mut value = 0_u64;
    for (index, byte) in data
        .get(at..)
        .unwrap_or_default()
        .iter()
        .enumerate()
        .take(10)
    {
        value |= u64::from(byte & 0x7f) << (7 * index);
        if byte & 0x80 == 0 {
            return Ok((value, index + 1));
        }
    }
    bail!("truncated LEB128 number")
}
