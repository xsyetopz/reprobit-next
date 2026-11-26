//! Register renaming in compiled i386 objects: the one post-compile rewrite
//! `ReproBit` keeps from the Python engine's recipe families.
//!
//! Compilers of every era settle ties in register allocation differently,
//! so a function can compile to the reference except that `esi` and `edi`
//! trade places. A permutation of the registers every i386 calling
//! convention preserves (`ebx`, `esi`, `edi`) applied to a whole function
//! cannot change what the function does: its callers expect those registers
//! back unchanged, which the renamed saves and restores still guarantee, and
//! no caller or callee reads them as arguments or results.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use iced_x86::{
    Decoder, DecoderOptions, Encoder, FlowControl, InstructionInfoFactory, OpKind, Register,
};
use object::{
    Architecture, BinaryFormat, Object, ObjectSection, ObjectSymbol, RelocationTarget,
    SectionIndex, SymbolIndex, SymbolKind,
};

use crate::binary::{self, Function};

mod codeview;
mod dwarf;

/// Renamed registers, as `{ from = "to" }`.
pub(crate) type Registers = BTreeMap<String, String>;

/// The registers a function in `object_bytes` may permute. i386 ELF leaves
/// `ebx` out, because position-independent code calls through the PLT with
/// the GOT address in it.
pub(crate) fn renamable(object_bytes: &[u8]) -> Result<&'static [&'static str]> {
    let file = object::File::parse(object_bytes)?;
    ensure!(
        file.architecture() == Architecture::I386,
        "register renaming covers i386 objects; this one is {:?}",
        file.architecture()
    );
    Ok(match file.format() {
        BinaryFormat::Elf => &["esi", "edi"],
        BinaryFormat::Coff => &["ebx", "esi", "edi"],
        // Mach-O compact unwind records saved registers in a form that is
        // not renamed.
        format => bail!("register renaming covers COFF and ELF objects; this one is {format:?}"),
    })
}

/// Renames registers in functions of an object, code and debug records
/// alike: `CodeView` symbols and frame programs, DWARF locations, and
/// call-frame information, so debuggers and unwinders follow the code.
pub(crate) fn rename_object(
    mut bytes: Vec<u8>,
    functions: &BTreeMap<String, Registers>,
) -> Result<Vec<u8>> {
    let allowed = renamable(&bytes)?;
    for (symbol, registers) in functions {
        let context = || format!("cannot rename registers in {symbol}");
        let (at, function) = binary::locate(&bytes, symbol)?;
        let renamed = rename(&function, registers, allowed).with_context(context)?;
        bytes[at..at + renamed.len()].copy_from_slice(&renamed);
        let anchor = Anchor::find(&bytes, symbol)?;
        bytes = codeview::rename(bytes, &anchor, registers).with_context(context)?;
        dwarf::rename(&mut bytes, symbol, &anchor, registers).with_context(context)?;
    }
    Ok(bytes)
}

/// Where a function starts, so debug records can be tied to it through the
/// relocations that point at it.
struct Anchor {
    symbol: SymbolIndex,
    section: SectionIndex,
    offset: u64,
}

impl Anchor {
    fn find(bytes: &[u8], name: &str) -> Result<Self> {
        let file = object::File::parse(bytes)?;
        let symbol = file
            .symbols()
            .find(|symbol| symbol.name() == Ok(name))
            .with_context(|| format!("the object has no symbol {name}"))?;
        let section = symbol
            .section_index()
            .context("the symbol is not defined in a section")?;
        Ok(Self {
            symbol: symbol.index(),
            section,
            offset: symbol.address() - file.section_by_index(section)?.address(),
        })
    }

    /// Offsets in `section` of relocations that point at the function's
    /// first byte, through its symbol or its section's.
    fn hits(
        &self,
        file: &object::File<'_>,
        section: &object::Section<'_, '_>,
    ) -> Result<Vec<usize>> {
        let data = section.data()?;
        let mut hits = Vec::new();
        for (offset, relocation) in section.relocations() {
            let RelocationTarget::Symbol(target) = relocation.target() else {
                continue;
            };
            let at = usize::try_from(offset)?;
            let addend = if relocation.has_implicit_addend() {
                data.get(at..at + 4).map_or(0, |field| {
                    u64::from(u32::from_le_bytes([field[0], field[1], field[2], field[3]]))
                })
            } else {
                relocation.addend().cast_unsigned()
            };
            let hit = if target == self.symbol {
                addend == 0
            } else {
                file.symbol_by_index(target).is_ok_and(|symbol| {
                    symbol.kind() == SymbolKind::Section
                        && symbol.section_index() == Some(self.section)
                }) && addend == self.offset
            };
            if hit {
                hits.push(at);
            }
        }
        Ok(hits)
    }
}

/// Replaces COFF section `index` with `data`, which grew by `by` bytes at
/// section offset `at`. The new data goes where the symbol table was, the
/// symbol table after it, and the section's relocations past `at` move with
/// their bytes; the old data stays behind, unreferenced.
fn coff_grow(
    mut bytes: Vec<u8>,
    index: SectionIndex,
    data: Vec<u8>,
    at: usize,
    by: usize,
) -> Result<Vec<u8>> {
    let word = |bytes: &[u8], at: usize| -> Result<usize> {
        let field = bytes.get(at..at + 4).context("truncated COFF header")?;
        Ok(usize::try_from(u32::from_le_bytes([
            field[0], field[1], field[2], field[3],
        ]))?)
    };
    let put = |bytes: &mut [u8], at: usize, value: usize| -> Result<()> {
        bytes[at..at + 4].copy_from_slice(&u32::try_from(value)?.to_le_bytes());
        Ok(())
    };
    let symbols = word(&bytes, 8)?;
    let optional = usize::from(u16::from_le_bytes([bytes[16], bytes[17]]));
    let count = usize::from(u16::from_le_bytes([bytes[2], bytes[3]]));
    ensure!(symbols > 0, "the object has no symbol table to move");
    for header in (0..count).map(|number| 20 + optional + 40 * number) {
        for field in [20, 24, 28] {
            ensure!(
                word(&bytes, header + field)? <= symbols,
                "a section lies after the symbol table"
            );
        }
    }
    let header = 20 + optional + 40 * (index.0 - 1);
    let relocations = word(&bytes, header + 24)?;
    let relocation_count =
        usize::from(u16::from_le_bytes([bytes[header + 32], bytes[header + 33]]));
    ensure!(
        word(&bytes, header + 36)? & 0x0100_0000 == 0,
        "the section has more than 65535 relocations"
    );
    for entry in (0..relocation_count).map(|number| relocations + 10 * number) {
        let offset = word(&bytes, entry)?;
        if offset >= at {
            put(&mut bytes, entry, offset + by)?;
        }
    }
    put(&mut bytes, header + 16, data.len())?;
    put(&mut bytes, header + 20, symbols)?;
    put(&mut bytes, 8, symbols + data.len())?;
    bytes.splice(symbols..symbols, data);
    Ok(bytes)
}

/// Every non-identity permutation of `names`, each without its fixed points.
pub(crate) fn permutations(names: &[&str]) -> Vec<Registers> {
    let mut orders: Vec<Vec<&str>> = vec![Vec::new()];
    for name in names {
        orders = orders
            .into_iter()
            .flat_map(|order| {
                (0..=order.len()).map(move |at| {
                    let mut next = order.clone();
                    next.insert(at, name);
                    next
                })
            })
            .collect();
    }
    orders
        .into_iter()
        .map(|order| {
            names
                .iter()
                .zip(order)
                .filter(|(from, to)| *from != to)
                .map(|(from, to)| ((*from).to_owned(), to.to_owned()))
                .collect::<Registers>()
        })
        .filter(|map| !map.is_empty())
        .collect()
}

/// Returns `function`'s bytes with `registers` renamed. It refuses a map
/// that is not a permutation of `allowed`, and any instruction it cannot
/// rename in place: one that uses a renamed register implicitly, such as
/// `rep movsd`, a byte register the target lacks, or an encoding whose length
/// would change. Relocated fields keep their bytes.
pub(crate) fn rename(
    function: &Function,
    registers: &Registers,
    allowed: &[&str],
) -> Result<Vec<u8>> {
    let table = registers
        .iter()
        .map(|(from, to)| {
            ensure!(
                allowed.contains(&from.as_str()) && allowed.contains(&to.as_str()) && from != to,
                "cannot rename {from} to {to}; a rename permutes {}",
                allowed.join(", ")
            );
            Ok((register(from)?, register(to)?))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    ensure!(
        table.keys().collect::<BTreeSet<_>>() == table.values().collect(),
        "the renamed registers {registers:?} are not a permutation"
    );
    let swap = |used: Register| -> Result<Register> {
        let Some(&to) = table.get(&used.full_register32()) else {
            return Ok(used);
        };
        match used.size() {
            4 => Ok(to),
            2 => Ok(match to {
                Register::EBX => Register::BX,
                Register::ESI => Register::SI,
                _ => Register::DI,
            }),
            _ => bail!("the function uses {used:?}, which {to:?} has no form of"),
        }
    };
    let mut info = InstructionInfoFactory::new();
    let mut renamed = Vec::with_capacity(function.bytes.len());
    let mut decoder = Decoder::with_ip(32, &function.bytes, 0, DecoderOptions::NONE);
    for mut instruction in &mut decoder {
        let at = instruction.ip();
        ensure!(
            !instruction.is_invalid(),
            "cannot decode the instruction at {at:#x}"
        );
        // Compilers place switch tables, and MSVC their byte index tables,
        // after the code that jumps through them; those bytes are data.
        ensure!(
            instruction.flow_control() != FlowControl::IndirectBranch,
            "the function jumps through a table at {at:#x}, so data may follow its code"
        );
        // A permutation hides a symmetric implicit use, such as `movsd`
        // reading both `esi` and `edi`, from any before-and-after
        // comparison, so each renamed register must be named by an operand.
        let used: Vec<Register> = info
            .info(&instruction)
            .used_registers()
            .iter()
            .map(|used| used.register().full_register32())
            .collect();
        let mut named = BTreeSet::new();
        for operand in 0..instruction.op_count() {
            match instruction.op_kind(operand) {
                OpKind::Register => {
                    named.insert(instruction.op_register(operand).full_register32());
                    instruction.set_op_register(operand, swap(instruction.op_register(operand))?);
                }
                OpKind::Memory => {
                    named.extend(
                        [instruction.memory_base(), instruction.memory_index()]
                            .map(Register::full_register32),
                    );
                    instruction.set_memory_base(swap(instruction.memory_base())?);
                    instruction.set_memory_index(swap(instruction.memory_index())?);
                }
                _ => {}
            }
        }
        if let Some(hidden) = used
            .iter()
            .find(|used| table.contains_key(used) && !named.contains(used))
        {
            bail!("the instruction at {at:#x} uses {hidden:?} implicitly");
        }
        let mut encoder = Encoder::new(32);
        encoder
            .encode(&instruction, at)
            .with_context(|| format!("cannot encode the renamed instruction at {at:#x}"))?;
        let bytes = encoder.take_buffer();
        ensure!(
            bytes.len() == instruction.len(),
            "renaming changes the length of the instruction at {at:#x}"
        );
        renamed.extend(bytes);
    }
    for (offset, _) in &function.relocations {
        let field = usize::try_from(*offset)?
            ..usize::try_from(*offset)?
                .saturating_add(4)
                .min(renamed.len());
        ensure!(
            renamed.get(field.clone()) == function.bytes.get(field),
            "renaming changes the relocated field at {offset:#x}"
        );
    }
    Ok(renamed)
}

fn register(name: &str) -> Result<Register> {
    Ok(match name {
        "ebx" => Register::EBX,
        "esi" => Register::ESI,
        "edi" => Register::EDI,
        _ => bail!(
            "{name} cannot be renamed; only ebx, esi, and edi are preserved by every i386 calling convention"
        ),
    })
}
