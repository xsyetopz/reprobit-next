//! The check behind `rename` and `exchange`: Lean Theorem 3
//! (`proofs/ReproBit/Correspondence.lean`). `σ` says which new location
//! holds which old location's value at each instruction, over the general
//! registers and the flags. It is the greatest fixpoint over the control
//! flow graph, so `σ` at a successor is within what holds after its
//! predecessor (`Checked.edge`). Each instruction must find its inputs in `σ`
//! (`Checked.reads`), and at each exit the registers a caller sees must hold
//! their old values (`Checked.exit`). A rewritten instruction may write some
//! locations with other values than the original, its clobbers, as long as
//! nothing reads them before they are written again.
//!
//! A call reads `ecx`, `edx`, and the stack, clobbers `eax`, `ecx`, `edx`,
//! and the flags, and the callee returns `ebx`, `esi`, `edi`, and `ebp`
//! unchanged: the i386 calling conventions, which `rbit`'s whole-function
//! renames also assume. No caller reads the flags a function leaves.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use iced_x86::{
    Decoder, DecoderOptions, FlowControl, Instruction, InstructionInfoFactory, Mnemonic, OpAccess,
    OpKind, Register,
};

use crate::coff::Function;
use crate::schedule::Location;

/// The general registers, as their 32-bit forms.
pub(crate) const GENERAL: [Register; 8] = [
    Register::EAX,
    Register::ECX,
    Register::EDX,
    Register::EBX,
    Register::ESP,
    Register::EBP,
    Register::ESI,
    Register::EDI,
];
/// What a caller sees on return.
const RETURNED: [Register; 7] = [
    Register::EAX,
    Register::EDX,
    Register::EBX,
    Register::ESP,
    Register::EBP,
    Register::ESI,
    Register::EDI,
];
/// `iced-x86`'s flag bits run below this.
const FLAG_BITS: u32 = 16;

/// Pairs `(new, old)`: the new program's `new` holds the old one's `old`.
pub(crate) type Sigma = BTreeSet<(Location, Location)>;

/// Decodes `code`, refusing bytes that do not decode.
pub(crate) fn decode(code: &[u8]) -> Result<Vec<Instruction>> {
    Decoder::with_ip(32, code, 0, DecoderOptions::NONE)
        .iter()
        .map(|instruction| {
            ensure!(
                !instruction.is_invalid(),
                "cannot decode the instruction at {:#x}",
                instruction.ip()
            );
            Ok(instruction)
        })
        .collect()
}

/// Decodes the one instruction `bytes` hold, placed at `ip`.
pub(crate) fn decode_at(bytes: &[u8], ip: u64) -> Result<Instruction> {
    let decoded: Vec<Instruction> = Decoder::with_ip(32, bytes, ip, DecoderOptions::NONE)
        .iter()
        .collect();
    match decoded.as_slice() {
        [one] if !one.is_invalid() => Ok(*one),
        _ => bail!("the bytes at {ip:#x} do not decode as one instruction"),
    }
}

/// The relocated fields, from `fields`, that start inside `instruction`.
pub(crate) fn fields(instruction: &Instruction, fields: &[usize]) -> Vec<usize> {
    let start = usize::try_from(instruction.ip()).unwrap_or(usize::MAX);
    fields
        .iter()
        .copied()
        .filter(|field| (start..start + instruction.len()).contains(field))
        .collect()
}

/// An instruction's form: its operation, length, immediates, branch
/// target, and any displacement off a register other than `esp`, without
/// the registers it names.
pub(crate) fn form(instruction: &Instruction) -> String {
    let operands: Vec<u64> = (0..instruction.op_count())
        .filter_map(|operand| match instruction.op_kind(operand) {
            OpKind::Memory if instruction.memory_base() != Register::ESP => {
                Some(instruction.memory_displacement64())
            }
            OpKind::NearBranch16 | OpKind::NearBranch32 => Some(instruction.near_branch_target()),
            OpKind::Register | OpKind::Memory => None,
            _ => instruction.try_immediate(operand).ok(),
        })
        .collect();
    format!(
        "{:?} {} {operands:?}",
        instruction.code(),
        instruction.len()
    )
}

/// The indices of the instructions that code outside `function` enters.
pub(crate) fn entries(decoded: &[Instruction], function: &Function) -> Result<Vec<usize>> {
    function
        .entries
        .iter()
        .map(|&entry| {
            decoded
                .iter()
                .position(|instruction| u64::try_from(entry).ok() == Some(instruction.ip()))
                .with_context(|| {
                    format!("{entry:#x} is entered from outside but starts no instruction")
                })
        })
        .collect()
}

/// The flags in an `iced-x86` flag mask.
pub(crate) fn flags(bits: u32) -> impl Iterator<Item = Location> {
    (0..FLAG_BITS)
        .filter(move |bit| bits >> bit & 1 == 1)
        .map(Location::Flag)
}

/// The general registers and flags an instruction reads and writes. A
/// partial or conditional write also reads, and so does a shift whose count
/// may be zero, which leaves the flags alone. Calls follow the calling
/// convention.
pub(crate) fn footprint(
    info: &mut InstructionInfoFactory,
    instruction: &Instruction,
) -> (BTreeSet<Location>, BTreeSet<Location>) {
    let register = Location::Register;
    if matches!(
        instruction.flow_control(),
        FlowControl::Call | FlowControl::IndirectCall
    ) {
        let mut reads: BTreeSet<Location> = [Register::ECX, Register::EDX, Register::ESP]
            .into_iter()
            .chain(explicit(instruction))
            .map(register)
            .collect();
        reads.extend(flags(instruction.rflags_read()));
        let mut writes: BTreeSet<Location> =
            [Register::EAX, Register::ECX, Register::EDX, Register::ESP]
                .into_iter()
                .map(register)
                .collect();
        writes.extend(flags(u32::MAX));
        return (reads, writes);
    }
    let (mut reads, mut writes) = (BTreeSet::new(), BTreeSet::new());
    for used in info.info(instruction).used_registers() {
        let full = used.register().full_register32();
        if !GENERAL.contains(&full) {
            continue;
        }
        let access = used.access();
        let write = matches!(
            access,
            OpAccess::Write | OpAccess::CondWrite | OpAccess::ReadWrite | OpAccess::ReadCondWrite
        );
        if matches!(
            access,
            OpAccess::Read
                | OpAccess::CondRead
                | OpAccess::CondWrite
                | OpAccess::ReadWrite
                | OpAccess::ReadCondWrite
        ) || (write && used.register().size() < 4)
        {
            reads.insert(register(full));
        }
        if write {
            writes.insert(register(full));
        }
    }
    reads.extend(flags(instruction.rflags_read()));
    writes.extend(flags(instruction.rflags_modified()));
    if may_keep_flags(instruction) {
        reads.extend(flags(instruction.rflags_modified()));
    }
    (reads, writes)
}

/// A shift or rotate by a count that may be zero leaves the flags as they were.
fn may_keep_flags(instruction: &Instruction) -> bool {
    let shift = matches!(
        instruction.mnemonic(),
        Mnemonic::Shl
            | Mnemonic::Sal
            | Mnemonic::Shr
            | Mnemonic::Sar
            | Mnemonic::Rol
            | Mnemonic::Ror
            | Mnemonic::Rcl
            | Mnemonic::Rcr
            | Mnemonic::Shld
            | Mnemonic::Shrd
    );
    let last = instruction.op_count().saturating_sub(1);
    let counted =
        instruction.op_kind(last) == OpKind::Immediate8 && instruction.immediate(last) & 0x1f != 0;
    shift && !counted
}

/// The 32-bit registers an instruction's operands name.
pub(crate) fn explicit(instruction: &Instruction) -> BTreeSet<Register> {
    (0..instruction.op_count())
        .flat_map(|operand| match instruction.op_kind(operand) {
            OpKind::Register => vec![instruction.op_register(operand)],
            OpKind::Memory => vec![instruction.memory_base(), instruction.memory_index()],
            _ => Vec::new(),
        })
        .map(Register::full_register32)
        .filter(|register| GENERAL.contains(register))
        .collect()
}

/// One point of the control flow graph: an instruction, or a run of them
/// rewritten together, entered only at its first.
pub(crate) struct Point {
    pub(crate) reads: BTreeSet<Location>,
    pub(crate) writes: BTreeSet<Location>,
    /// The writes whose values both programs agree on (Lean `kept`).
    pub(crate) kept: BTreeSet<Location>,
    pub(crate) rho: BTreeMap<Register, Register>,
    pub(crate) next: Vec<usize>,
    /// The registers that must hold their old values after an exit.
    pub(crate) exit: Vec<Register>,
}

impl Point {
    /// The instruction at `index`, renamed under `rho`.
    pub(crate) fn new(
        info: &mut InstructionInfoFactory,
        decoded: &[Instruction],
        index: usize,
        relocated: bool,
        rho: BTreeMap<Register, Register>,
    ) -> Result<Self> {
        let (reads, writes) = footprint(info, &decoded[index]);
        let (next, exit) = successors(decoded, index, relocated)?;
        Ok(Self {
            kept: writes.clone(),
            reads,
            writes,
            rho,
            next,
            exit,
        })
    }

    /// The instructions `first..=last`, rewritten together so that they
    /// write `clobbers` with values of their own.
    pub(crate) fn run(
        points: &[Self],
        first: usize,
        last: usize,
        clobbers: &BTreeSet<Location>,
    ) -> Self {
        let (mut reads, mut writes) = (BTreeSet::new(), BTreeSet::new());
        for point in &points[first..=last] {
            reads.extend(point.reads.difference(&writes).copied());
            writes.extend(&point.writes);
        }
        Self {
            kept: writes.difference(clobbers).copied().collect(),
            reads,
            writes,
            rho: BTreeMap::new(),
            next: points[last].next.clone(),
            exit: points[last].exit.clone(),
        }
    }

    fn rho(&self, location: Location) -> Location {
        match location {
            Location::Register(register) => {
                Location::Register(self.rho.get(&register).copied().unwrap_or(register))
            }
            other => other,
        }
    }

    /// Lean `Post`: the pairs that hold after this point.
    fn post(&self, sigma: &Sigma) -> Sigma {
        let renamed: BTreeSet<Location> =
            self.writes.iter().map(|&write| self.rho(write)).collect();
        sigma
            .iter()
            .filter(|(new, old)| !renamed.contains(new) && !self.writes.contains(old))
            .copied()
            .chain(self.kept.iter().map(|&write| (self.rho(write), write)))
            .collect()
    }
}

/// Where control goes after instruction `index`, and what must hold if it
/// leaves the function. A relocated branch leaves it.
fn successors(
    decoded: &[Instruction],
    index: usize,
    relocated: bool,
) -> Result<(Vec<usize>, Vec<Register>)> {
    let instruction = &decoded[index];
    let target = (!relocated)
        .then(|| {
            decoded
                .iter()
                .position(|other| other.ip() == instruction.near_branch_target())
        })
        .flatten();
    let following = (index + 1 < decoded.len()).then_some(index + 1);
    Ok(match instruction.flow_control() {
        FlowControl::Next | FlowControl::Call | FlowControl::IndirectCall => match following {
            Some(next) => (vec![next], Vec::new()),
            None => (Vec::new(), GENERAL.to_vec()),
        },
        FlowControl::Return => (Vec::new(), RETURNED.to_vec()),
        FlowControl::ConditionalBranch => match (target, following) {
            (Some(target), Some(next)) => (vec![target, next], Vec::new()),
            _ => bail!(
                "the branch at {:#x} leaves the function, which this check does not follow",
                instruction.ip()
            ),
        },
        // A jump out of the function is a tail call.
        FlowControl::UnconditionalBranch => match target {
            Some(target) => (vec![target], Vec::new()),
            None => (Vec::new(), GENERAL.to_vec()),
        },
        FlowControl::IndirectBranch => bail!(
            "the function jumps through a table at {:#x}, so data may follow its code",
            instruction.ip()
        ),
        _ => (Vec::new(), GENERAL.to_vec()),
    })
}

/// Lean `Checked`: computes `σ` from the identity at the function's start
/// and at each of `entries`, the instructions entered from outside, and
/// checks reads and exits.
pub(crate) fn check(points: &[Point], decoded: &[Instruction], entries: &[usize]) -> Result<()> {
    if points.is_empty() {
        return Ok(());
    }
    let identity: Sigma = GENERAL
        .iter()
        .map(|&register| Location::Register(register))
        .chain(flags(u32::MAX))
        .map(|location| (location, location))
        .collect();
    let mut sigma: Vec<Option<Sigma>> = vec![None; points.len()];
    let mut work = Vec::new();
    for &entry in std::iter::once(&0).chain(entries) {
        sigma[entry] = Some(identity.clone());
        work.push(entry);
    }
    while let Some(index) = work.pop() {
        let Some(current) = &sigma[index] else {
            continue;
        };
        let after = points[index].post(current);
        for &next in &points[index].next {
            let met = match &sigma[next] {
                None => after.clone(),
                Some(old) => old.intersection(&after).copied().collect(),
            };
            if sigma[next].as_ref() != Some(&met) {
                sigma[next] = Some(met);
                work.push(next);
            }
        }
    }
    for (index, point) in points.iter().enumerate() {
        let Some(current) = &sigma[index] else {
            continue;
        };
        let at = decoded[index].ip();
        if let Some(&read) = point
            .reads
            .iter()
            .find(|&&read| !current.contains(&(point.rho(read), read)))
        {
            bail!(
                "the instruction at {at:#x} would read {} where the original read {}, which may hold another value there",
                name(point.rho(read)),
                name(read)
            );
        }
        let after = point.post(current);
        if let Some(register) = point.exit.iter().find(|&&register| {
            let location = Location::Register(register);
            !after.contains(&(location, location))
        }) {
            bail!("{register:?} would hold another value when the function leaves at {at:#x}");
        }
    }
    Ok(())
}

/// A location as an error message names it.
fn name(location: Location) -> String {
    const FLAGS: [&str; 6] = ["OF", "SF", "ZF", "AF", "CF", "PF"];
    match location {
        Location::Register(register) => format!("{register:?}"),
        Location::Flag(bit) => usize::try_from(bit)
            .ok()
            .and_then(|bit| FLAGS.get(bit))
            .map_or_else(|| format!("flag {bit}"), |name| (*name).to_owned()),
        other => format!("{other:?}"),
    }
}
