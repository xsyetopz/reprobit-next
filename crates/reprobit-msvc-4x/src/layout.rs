//! Writes a function's instructions back when some changed length: each
//! moves to its new place, relative branches inside the function are
//! re-encoded for their targets' new places, and relocated fields follow
//! the displacement or immediate that holds them. Unreachable padding after
//! the function's last jump or return absorbs the change in length where it
//! can, so the functions after it keep their places; otherwise the object
//! is laid out again around the new length.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail, ensure};
use iced_x86::{Code, Encoder, FlowControl, Instruction};

use crate::coff::{Coff, Function};
use crate::correspond;

/// The object `object` with `function`'s instructions, `decoded`, written
/// in their new forms, `encoded`.
pub(crate) fn lay_out(
    object: &[u8],
    function: &Function,
    decoded: &[Instruction],
    mut encoded: Vec<Vec<u8>>,
) -> Result<Vec<u8>> {
    let fields = function.fields();
    let old: Vec<usize> = decoded
        .iter()
        .map(|instruction| usize::try_from(instruction.ip()))
        .collect::<Result<_, _>>()?;
    let length = function.bytes.len();
    let total = |encoded: &[Vec<u8>]| encoded.iter().map(Vec::len).sum::<usize>();
    let padding = padding(function, decoded, &old);
    let mut kept = encoded.len();
    if let Some(first) = padding {
        let grown = total(&encoded);
        if grown > length && length - old[first] >= grown - length {
            kept -= grown - length;
            encoded.truncate(kept);
        } else if grown < length {
            let filler = encoded[first].clone();
            encoded.extend((grown..length).map(|_| filler.clone()));
        }
    }
    let mut new = Vec::with_capacity(encoded.len());
    let mut at = 0;
    for bytes in &encoded {
        new.push(at);
        at += bytes.len();
    }
    let mut place = BTreeMap::from([(length, at)]);
    for (index, &start) in old.iter().enumerate() {
        place.insert(
            start,
            new.get(index)
                .copied()
                .filter(|_| index < kept)
                .unwrap_or(at),
        );
    }
    let mut moved = BTreeMap::new();
    for (index, instruction) in decoded.iter().enumerate().take(kept) {
        let inside = correspond::fields(instruction, &fields);
        if matches!(
            instruction.flow_control(),
            FlowControl::ConditionalBranch | FlowControl::UnconditionalBranch | FlowControl::Call
        ) && inside.is_empty()
        {
            let target = usize::try_from(instruction.near_branch_target())?;
            let Some(&to) = place.get(&target).filter(|_| target < length) else {
                ensure!(
                    new[index] == old[index] && at == length,
                    "the branch at {:#x} leaves the function without a relocation, so it cannot move",
                    old[index]
                );
                continue;
            };
            let mut branch = *instruction;
            branch.set_near_branch32(u32::try_from(to)?);
            let mut encoder = Encoder::new(32);
            let written = encoder
                .encode(&branch, u64::try_from(new[index])?)
                .with_context(|| {
                    format!("the branch at {:#x} cannot reach its target", old[index])
                })?;
            ensure!(
                written == encoded[index].len(),
                "the branch at {:#x} would need a longer form",
                old[index]
            );
            encoded[index] = encoder.take_buffer();
        }
        for field in inside {
            let offset = field - old[index];
            let within = if encoded[index].len() == instruction.len() {
                offset
            } else {
                counterpart(&function.bytes[old[index]..], &encoded[index], offset)?
            };
            ensure!(
                encoded[index].get(within..within + 4) == function.bytes.get(field..field + 4),
                "the relocated field at {field:#x} would change"
            );
            moved.insert(field, new[index] + within);
        }
    }
    let code = encoded.concat();
    Coff::parse(object)?.replace(function, &code, &place, &moved)
}

/// The first of the padding instructions at the function's end: `int3` or
/// `nop` after its last jump or return, which nothing branches into.
fn padding(function: &Function, decoded: &[Instruction], starts: &[usize]) -> Option<usize> {
    let first = decoded
        .iter()
        .rposition(|instruction| !matches!(instruction.code(), Code::Int3 | Code::Nopd))?
        + 1;
    let targets: Vec<u64> = decoded
        .iter()
        .map(Instruction::near_branch_target)
        .collect();
    let ends = matches!(
        decoded[first - 1].flow_control(),
        FlowControl::Return | FlowControl::UnconditionalBranch
    );
    let entered = starts[first..].iter().any(|&start| {
        targets.contains(&u64::try_from(start).unwrap_or(u64::MAX))
            || function.barriers.contains(&start)
            || function.entries.contains(&start)
    });
    (first < decoded.len() && ends && !entered).then_some(first)
}

/// Where the field at `offset` in the instruction `old` starts lies in its
/// new encoding `new`: the same displacement or immediate.
fn counterpart(old: &[u8], new: &[u8], offset: usize) -> Result<usize> {
    let (before, after) = (correspond::constants(old), correspond::constants(new));
    let pairs = [
        (before.displacement_offset(), after.displacement_offset()),
        (before.immediate_offset(), after.immediate_offset()),
        (before.immediate_offset2(), after.immediate_offset2()),
    ];
    match pairs.iter().find(|(old, _)| *old == offset) {
        Some(&(_, new)) => Ok(new),
        None => bail!("a relocated field is neither a displacement nor an immediate"),
    }
}
