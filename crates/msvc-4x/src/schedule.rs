//! Instruction footprints and the reordering check. The check is the Lean
//! predicate `reorderOk` from `proofs/ReproBit/Reorder.lean`: an order is
//! accepted when it is a permutation of the instructions and every pair it
//! inverts does not conflict. `reorderOk_sound` proves the reordered
//! function computes the same state, given footprints that list every
//! location an instruction writes and every location its writes depend on.
//!
//! Stack slots are locations of their own, numbered by their offset from
//! `esp` at the last barrier. An `[esp+d]` operand names its slot, not
//! `esp`, so it may cross a `push`, and its displacement is rebased to
//! address the same slot from its new place. Pushes, pops, and `add` or
//! `sub esp` by a constant keep their order among themselves, which keeps
//! each one's own offset.

use std::collections::BTreeSet;

use anyhow::{Result, bail, ensure};
use iced_x86::{
    CpuidFeature, Decoder, DecoderOptions, FlowControl, Instruction as Decoded,
    InstructionInfoFactory, Mnemonic, OpAccess, OpKind, Register, UsedMemory,
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
    /// One stack byte, by run and by offset from `esp` at the run's start.
    Slot(usize, i64),
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
    barrier: bool,
    /// `esp`'s offset from its value at the run's start, before this instruction.
    depth: i64,
    /// How far the instruction moves `esp`.
    adjust: i64,
    /// The `[esp+d]` displacement field: offset, width, and value.
    displacement: Option<(usize, usize, i64)>,
}

impl Instruction {
    /// The stack slot an `[esp+d]` operand names, which is what a
    /// proposal matches instead of its displacement.
    pub(crate) fn slot(&self) -> Option<i64> {
        self.displacement.map(|(_, _, value)| self.depth + value)
    }

    /// The bytes with the displacement field cleared.
    pub(crate) fn shape(&self) -> Vec<u8> {
        let mut bytes = self.bytes.clone();
        if let Some((at, width, _)) = self.displacement {
            bytes[at..at + width].fill(0);
        }
        bytes
    }
}

/// A memory displacement as the signed 32-bit value it encodes.
fn signed(value: u64) -> i64 {
    i64::from(i32::from_le_bytes(
        u32::try_from(value & 0xffff_ffff)
            .unwrap_or_default()
            .to_le_bytes(),
    ))
}

/// Decodes `code` and computes each instruction's footprint. Instructions
/// that start at a `barriers` offset, transfer control, or touch state the
/// footprint does not model, such as I/O ports or `esp` set from a
/// register, become barriers.
pub(crate) fn decode(code: &[u8], barriers: &BTreeSet<usize>) -> Result<Vec<Instruction>> {
    let mut decoder = Decoder::with_ip(32, code, 0, DecoderOptions::NONE);
    let mut info = InstructionInfoFactory::new();
    let mut raw = Vec::new();
    let mut targets = BTreeSet::new();
    let mut instruction = Decoded::default();
    while decoder.can_decode() {
        decoder.decode_out(&mut instruction);
        let offsets = decoder.get_constant_offsets(&instruction);
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
        let field = (offsets.displacement_offset(), offsets.displacement_size());
        raw.push((
            instruction,
            field,
            used.used_registers().to_vec(),
            used.used_memory().to_vec(),
        ));
    }
    let mut decoded: Vec<Instruction> = Vec::with_capacity(raw.len());
    let (mut run, mut depth) = (0, 0);
    // Accesses through other registers may reach any stack byte.
    let mut aliasing = Vec::new();
    for (instruction, (at, width), registers, memory) in raw {
        let start = usize::try_from(instruction.ip())?;
        let (mut reads, mut writes) = (BTreeSet::from([Location::Control]), BTreeSet::new());
        let control = instruction.flow_control() != FlowControl::Next;
        let adjust = adjustment(&instruction);
        let stack_operand = stack_operand(&instruction);
        let displacement = stack_operand.then(|| {
            (
                at,
                width,
                signed(u64::from(instruction.memory_displacement32())),
            )
        });
        let mut moves_esp = false;
        for register in &registers {
            let full = Location::Register(register.register().full_register32());
            let (read, write) = access(register.access());
            moves_esp |= write && full == Location::Register(Register::ESP);
            // An `[esp+d]` operand is rebased when it moves, so it does not depend on `esp`.
            let rebased =
                stack_operand && adjust.is_none() && full == Location::Register(Register::ESP);
            // A partial write keeps the rest of the register, so it depends on it.
            if !rebased && (read || (write && register.register().size() < 4)) {
                reads.insert(full);
            }
            if write {
                writes.insert(full);
            }
        }
        let through = memory_footprint(&memory, run, depth, &mut reads, &mut writes)?;
        status_footprint(&instruction, &mut reads, &mut writes);
        let barrier = control
            || instruction.is_privileged()
            || instruction.has_lock_prefix()
            || barriers.contains(&start)
            || targets.contains(&start)
            || (moves_esp && adjust.is_none());
        if barrier {
            writes.insert(Location::Control);
        }
        aliasing.push(through);
        decoded.push(Instruction {
            start,
            bytes: code[start..start + instruction.len()].to_vec(),
            reads,
            writes,
            barrier,
            depth,
            adjust: adjust.unwrap_or_default(),
            displacement,
        });
        depth += adjust.unwrap_or_default();
        if barrier {
            (run, depth) = (run + 1, 0);
        }
    }
    alias(&mut decoded, aliasing);
    Ok(decoded)
}

/// Adds every stack byte the function names to each access through a
/// register other than `esp`, marked in `aliasing` as `(reads, writes)`.
fn alias(decoded: &mut [Instruction], aliasing: Vec<(bool, bool)>) {
    let slots: BTreeSet<Location> = decoded
        .iter()
        .flat_map(|item| item.reads.iter().chain(&item.writes))
        .filter(|location| matches!(location, Location::Slot(..)))
        .copied()
        .collect();
    for (item, (read, write)) in decoded.iter_mut().zip(aliasing) {
        if read {
            item.reads.extend(&slots);
        }
        if write {
            item.writes.extend(&slots);
        }
    }
}

/// How far `instruction` moves `esp`, when it moves it by a constant:
/// pushes, pops, and `add` or `sub esp` by an immediate.
fn adjustment(instruction: &Decoded) -> Option<i64> {
    if instruction.stack_pointer_increment() != 0 && instruction.flow_control() == FlowControl::Next
    {
        return Some(i64::from(instruction.stack_pointer_increment()));
    }
    let by_immediate = instruction.op0_kind() == OpKind::Register
        && instruction.op0_register() == Register::ESP
        && matches!(
            instruction.op1_kind(),
            OpKind::Immediate8to32 | OpKind::Immediate32
        );
    let amount = || signed(instruction.immediate(1));
    match instruction.mnemonic() {
        Mnemonic::Add if by_immediate => Some(amount()),
        Mnemonic::Sub if by_immediate => Some(-amount()),
        _ => None,
    }
}

/// Whether `instruction`'s memory operand is `[esp+d]`, and it uses `esp`
/// nowhere else.
fn stack_operand(instruction: &Decoded) -> bool {
    let operands = 0..instruction.op_count();
    operands
        .clone()
        .any(|operand| instruction.op_kind(operand) == OpKind::Memory)
        && instruction.memory_base() == Register::ESP
        && instruction.memory_index() == Register::None
        && !operands.into_iter().any(|operand| {
            instruction.op_kind(operand) == OpKind::Register
                && instruction.op_register(operand) == Register::ESP
        })
}

/// Adds the memory an instruction accesses: the stack bytes `[esp+d]`
/// names in `run`, or all memory. Returns whether it reads or writes
/// through another register, which may reach any stack byte.
fn memory_footprint(
    memory: &[UsedMemory],
    run: usize,
    depth: i64,
    reads: &mut BTreeSet<Location>,
    writes: &mut BTreeSet<Location>,
) -> Result<(bool, bool)> {
    let mut through = (false, false);
    for memory in memory {
        let (read, write) = access(memory.access());
        if memory.base() == Register::ESP && memory.index() == Register::None {
            let first = depth + signed(memory.displacement());
            for byte in first..first + i64::try_from(memory.memory_size().size())? {
                if read || write {
                    reads.insert(Location::Slot(run, byte));
                }
                if write {
                    writes.insert(Location::Slot(run, byte));
                }
            }
        } else {
            through.0 |= read || write;
            through.1 |= write;
        }
    }
    if through.0 {
        reads.insert(Location::Memory);
    }
    if through.1 {
        writes.insert(Location::Memory);
    }
    Ok(through)
}

/// Adds the flags and the x87 and SSE state `instruction` uses.
fn status_footprint(
    instruction: &Decoded,
    reads: &mut BTreeSet<Location>,
    writes: &mut BTreeSet<Location>,
) {
    let flags = |bits: u32| {
        (0..32)
            .filter(move |bit| bits >> bit & 1 == 1)
            .map(Location::Flag)
    };
    reads.extend(flags(instruction.rflags_read()));
    writes.extend(flags(instruction.rflags_modified()));
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
}

/// Each instruction's bytes in `order`, with `[esp+d]` displacements
/// rebased to name the same slot from the new place.
pub(crate) fn emit(instructions: &[Instruction], order: &[usize]) -> Result<Vec<Vec<u8>>> {
    let mut depth = 0;
    let mut emitted = Vec::with_capacity(order.len());
    for &index in order {
        let instruction = &instructions[index];
        let mut bytes = instruction.bytes.clone();
        if let Some((at, width, value)) = instruction.displacement {
            let rebased = value + instruction.depth - depth;
            let field = &mut bytes[at..at + width];
            match width {
                0 if rebased == 0 => {}
                1 if let Ok(value) = i8::try_from(rebased) => {
                    field.copy_from_slice(&value.to_le_bytes());
                }
                4 if let Ok(value) = i32::try_from(rebased) => {
                    field.copy_from_slice(&value.to_le_bytes());
                }
                _ => bail!(
                    "the instruction at {:#x} would need displacement {rebased}, which its encoding cannot hold",
                    instruction.start
                ),
            }
        }
        emitted.push(bytes);
        depth += instruction.adjust;
        if instruction.barrier {
            depth = 0;
        }
    }
    Ok(emitted)
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
