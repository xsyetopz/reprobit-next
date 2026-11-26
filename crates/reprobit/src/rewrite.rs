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
use object::{Architecture, BinaryFormat, Object};

use crate::binary::Function;

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
    Ok(if file.format() == BinaryFormat::Elf {
        &["esi", "edi"]
    } else {
        &["ebx", "esi", "edi"]
    })
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
