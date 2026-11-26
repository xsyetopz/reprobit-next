//! Instruction footprints and the reordering check. The check is the Lean
//! predicate `reorderOk` from `proofs/ReproBit/Reorder.lean`: an order is
//! accepted when it is a permutation of the instructions and every pair it
//! inverts does not conflict. `reorderOk_sound` proves the reordered
//! function computes the same state, given footprints that list every
//! location an instruction writes and every location its writes depend on.

use std::collections::BTreeSet;

use anyhow::{Result, ensure};
use iced_x86::{
    CpuidFeature, Decoder, DecoderOptions, FlowControl, InstructionInfoFactory, OpAccess, Register,
};

/// A machine location an instruction reads or writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Location {
    /// A general register, as its full 32-bit form.
    Register(Register),
    /// One `EFLAGS` bit.
    Flag(u32),
    /// All of memory, so any two accesses where one writes conflict.
    Memory,
    /// The x87 and SSE state: stack top, status, and control words.
    Float,
    /// Program order itself. Every instruction reads it; a barrier writes
    /// it, which makes a barrier conflict with every other instruction.
    Control,
}

/// One decoded instruction and its footprint.
#[derive(Debug)]
pub(crate) struct Instruction {
    pub(crate) start: usize,
    pub(crate) bytes: Vec<u8>,
    pub(crate) reads: BTreeSet<Location>,
    pub(crate) writes: BTreeSet<Location>,
}

/// Decodes `code` and computes each instruction's footprint. Instructions
/// that start at a `barriers` offset, transfer control, or touch state the
/// footprint does not model, such as I/O ports, become barriers.
pub(crate) fn decode(code: &[u8], barriers: &BTreeSet<usize>) -> Result<Vec<Instruction>> {
    let mut decoder = Decoder::with_ip(32, code, 0, DecoderOptions::NONE);
    let mut info = InstructionInfoFactory::new();
    let mut decoded = Vec::new();
    let mut targets = BTreeSet::new();
    for instruction in &mut decoder {
        let start = usize::try_from(instruction.ip())?;
        ensure!(
            !instruction.is_invalid(),
            "cannot decode the instruction at {start:#x}"
        );
        ensure!(
            instruction.flow_control() != FlowControl::IndirectBranch,
            "the function jumps through a table at {start:#x}, so data may follow its code"
        );
        if matches!(
            instruction.flow_control(),
            FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch
        ) {
            targets.insert(usize::try_from(instruction.near_branch_target())?);
        }
        let used = info.info(&instruction);
        let (mut reads, mut writes) = (BTreeSet::from([Location::Control]), BTreeSet::new());
        for register in used.used_registers() {
            let full = Location::Register(register.register().full_register32());
            let (read, write) = access(register.access());
            // A partial write keeps the rest of the register, so it depends on it.
            if read || (write && register.register().size() < 4) {
                reads.insert(full);
            }
            if write {
                writes.insert(full);
            }
        }
        for memory in used.used_memory() {
            let (read, write) = access(memory.access());
            if read || write {
                reads.insert(Location::Memory);
            }
            if write {
                writes.insert(Location::Memory);
            }
        }
        let flags = |bits: u32| {
            (0..32)
                .filter(move |bit| bits >> bit & 1 == 1)
                .map(Location::Flag)
        };
        reads.extend(flags(instruction.rflags_read()));
        let written = instruction.rflags_modified();
        writes.extend(flags(written));
        // An undefined flag's value is not modeled, so it also depends on the old one.
        reads.extend(flags(instruction.rflags_undefined()));
        if instruction.cpuid_features().iter().any(|feature| {
            matches!(
                feature,
                CpuidFeature::FPU
                    | CpuidFeature::FPU287
                    | CpuidFeature::FPU387
                    | CpuidFeature::SSE
                    | CpuidFeature::SSE2
            )
        }) {
            reads.insert(Location::Float);
            writes.insert(Location::Float);
        }
        if instruction.flow_control() != FlowControl::Next
            || instruction.is_privileged()
            || instruction.has_lock_prefix()
            || barriers.contains(&start)
        {
            writes.insert(Location::Control);
        }
        decoded.push(Instruction {
            start,
            bytes: code[start..start + instruction.len()].to_vec(),
            reads,
            writes,
        });
    }
    // A branch target keeps its offset, so jumps into the function stay right.
    for item in &mut decoded {
        if targets.contains(&item.start) {
            item.writes.insert(Location::Control);
        }
    }
    Ok(decoded)
}

fn access(access: OpAccess) -> (bool, bool) {
    match access {
        OpAccess::Read | OpAccess::CondRead => (true, false),
        OpAccess::Write => (false, true),
        // A conditional write may keep the old value, so it depends on it.
        OpAccess::CondWrite | OpAccess::ReadWrite | OpAccess::ReadCondWrite => (true, true),
        _ => (false, false),
    }
}

/// Lean `Conflict`: one writes what the other reads or writes.
fn conflict(first: &Instruction, second: &Instruction) -> bool {
    first
        .writes
        .iter()
        .any(|location| second.reads.contains(location) || second.writes.contains(location))
        || second
            .writes
            .iter()
            .any(|location| first.reads.contains(location))
}

/// Lean `reorderOk`: `order` is a permutation of the instruction indices,
/// and every pair it puts out of program order does not conflict. Returns
/// the first offending pair.
pub(crate) fn check(instructions: &[Instruction], order: &[usize]) -> Result<()> {
    let mut sorted = order.to_vec();
    sorted.sort_unstable();
    ensure!(
        sorted.iter().copied().eq(0..instructions.len()),
        "the order must list each of the {} instructions once",
        instructions.len()
    );
    for (position, &first) in order.iter().enumerate() {
        for &second in &order[position + 1..] {
            ensure!(
                first < second || !conflict(&instructions[first], &instructions[second]),
                "the instructions at {:#x} and {:#x} depend on each other, so their order must stay",
                instructions[second].start,
                instructions[first].start
            );
        }
    }
    Ok(())
}
