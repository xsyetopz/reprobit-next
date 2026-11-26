//! `rename`: renames registers in chosen instructions, such as one value's
//! web or a stretch where two registers trade places. The check is Lean
//! Theorem 3 (`proofs/ReproBit/Correspondence.lean`): `σ` says which new
//! register holds which old register's value at each instruction. It is the
//! greatest fixpoint over the control flow graph, so `σ` at a successor is
//! within what holds after its predecessor (`Checked.edge`). Each renamed
//! instruction must find its inputs in `σ` (`Checked.reads`), and at each
//! exit the registers a caller sees must hold their old values
//! (`Checked.exit`).
//!
//! A call reads `ecx`, `edx`, and the stack and clobbers `eax`, `ecx`, and
//! `edx`, and the callee returns `ebx`, `esi`, `edi`, and `ebp` unchanged:
//! the i386 calling conventions, which `rbit`'s whole-function renames
//! also assume.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use iced_x86::{
    Decoder, DecoderOptions, Encoder, FlowControl, Instruction, InstructionInfoFactory, OpAccess,
    OpKind, Register,
};

/// Per instruction index, a rename of 32-bit registers.
pub(crate) type Map = BTreeMap<usize, BTreeMap<Register, Register>>;

const GENERAL: [Register; 8] = [
    Register::EAX,
    Register::ECX,
    Register::EDX,
    Register::EBX,
    Register::ESP,
    Register::EBP,
    Register::ESI,
    Register::EDI,
];
const WORDS: [Register; 8] = [
    Register::AX,
    Register::CX,
    Register::DX,
    Register::BX,
    Register::SP,
    Register::BP,
    Register::SI,
    Register::DI,
];
const LOW: [Register; 4] = [Register::AL, Register::CL, Register::DL, Register::BL];
const HIGH: [Register; 4] = [Register::AH, Register::CH, Register::DH, Register::BH];
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

/// Parses `eax`-style names.
pub(crate) fn register(name: &str) -> Result<Register> {
    GENERAL
        .into_iter()
        .find(|register| format!("{register:?}").eq_ignore_ascii_case(name))
        .with_context(|| format!("{name} is not a 32-bit general register"))
}

/// Parses `i:from:to,…`.
pub(crate) fn parse(text: &str) -> Result<Map> {
    let mut map = Map::new();
    for item in text.split(',') {
        let parts: Vec<&str> = item.trim().split(':').collect();
        let [index, from, to] = parts[..] else {
            bail!("{item} is not index:from:to");
        };
        let index = index
            .parse()
            .with_context(|| format!("{index} is not an instruction index"))?;
        map.entry(index)
            .or_default()
            .insert(register(from)?, register(to)?);
    }
    Ok(map)
}

/// Writes `map` as `parse` reads it.
pub(crate) fn format(map: &Map) -> String {
    map.iter()
        .flat_map(|(index, renames)| {
            renames
                .iter()
                .map(move |(from, to)| format!("{index}:{from:?}:{to:?}").to_lowercase())
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// The rename that turns `built` into `reference` when they differ only in
/// the registers their operands name, checked by applying it.
pub(crate) fn derive(built: &[u8], reference: &[u8], relocations: &[usize]) -> Option<Map> {
    let decode = |code| -> Vec<Instruction> {
        Decoder::with_ip(32, code, 0, DecoderOptions::NONE)
            .iter()
            .collect()
    };
    let (ours, theirs) = (decode(built), decode(reference));
    if built.len() != reference.len() || ours.len() != theirs.len() {
        return None;
    }
    let operands = |instruction: &Instruction| -> Vec<Register> {
        (0..instruction.op_count())
            .flat_map(|operand| match instruction.op_kind(operand) {
                OpKind::Register => vec![instruction.op_register(operand)],
                OpKind::Memory => vec![instruction.memory_base(), instruction.memory_index()],
                _ => Vec::new(),
            })
            .map(Register::full_register32)
            .collect()
    };
    let mut map = Map::new();
    for (index, (our, their)) in ours.iter().zip(&theirs).enumerate() {
        if our.len() != their.len() || our.mnemonic() != their.mnemonic() {
            return None;
        }
        let mut renames = BTreeMap::new();
        for (from, to) in operands(our).into_iter().zip(operands(their)) {
            if from != to && *renames.entry(from).or_insert(to) != to {
                return None;
            }
        }
        if !renames.is_empty() {
            map.insert(index, renames);
        }
    }
    let renamed = apply(built, relocations, &map).ok()?;
    (!map.is_empty() && renamed == reference).then_some(map)
}

/// The form of `to` the same size as `used`.
fn counterpart(used: Register, to: Register) -> Result<Register> {
    let index = GENERAL
        .iter()
        .position(|&full| full == to)
        .unwrap_or_default();
    let form = match used.size() {
        4 => Some(to),
        2 => Some(WORDS[index]),
        _ if LOW.contains(&used) => LOW.get(index).copied(),
        _ => HIGH.get(index).copied(),
    };
    form.with_context(|| format!("{used:?} has no counterpart in {to:?}"))
}

/// The general registers an instruction reads and writes. A partial or
/// conditional write also reads. Calls follow the calling convention.
fn footprint(
    info: &mut InstructionInfoFactory,
    instruction: &Instruction,
) -> (BTreeSet<Register>, BTreeSet<Register>) {
    if matches!(
        instruction.flow_control(),
        FlowControl::Call | FlowControl::IndirectCall
    ) {
        let mut reads = BTreeSet::from([Register::ECX, Register::EDX, Register::ESP]);
        reads.extend(explicit(instruction));
        let writes = BTreeSet::from([Register::EAX, Register::ECX, Register::EDX, Register::ESP]);
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
            reads.insert(full);
        }
        if write {
            writes.insert(full);
        }
    }
    (reads, writes)
}

/// The 32-bit registers an instruction's operands name.
fn explicit(instruction: &Instruction) -> BTreeSet<Register> {
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

struct Point {
    reads: BTreeSet<Register>,
    writes: BTreeSet<Register>,
    rho: BTreeMap<Register, Register>,
    next: Vec<usize>,
    /// The registers that must hold their old values after an exit.
    exit: Vec<Register>,
}

impl Point {
    fn rho(&self, register: Register) -> Register {
        self.rho.get(&register).copied().unwrap_or(register)
    }

    /// Lean `Post`: the pairs that hold after this instruction.
    fn post(&self, sigma: &BTreeSet<(Register, Register)>) -> BTreeSet<(Register, Register)> {
        let renamed: BTreeSet<Register> =
            self.writes.iter().map(|&write| self.rho(write)).collect();
        sigma
            .iter()
            .filter(|(new, old)| !renamed.contains(new) && !self.writes.contains(old))
            .copied()
            .chain(self.writes.iter().map(|&write| (self.rho(write), write)))
            .collect()
    }
}

/// Renames the registers `map` names in `code`, whose relocated fields
/// start at `relocations`, after checking the renamed code computes what
/// the original did.
pub(crate) fn apply(code: &[u8], relocations: &[usize], map: &Map) -> Result<Vec<u8>> {
    let decoded: Vec<Instruction> = Decoder::with_ip(32, code, 0, DecoderOptions::NONE)
        .iter()
        .collect();
    let starts = decoded
        .iter()
        .map(|instruction| usize::try_from(instruction.ip()))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(index) = map.keys().find(|&&index| index >= decoded.len()) {
        bail!(
            "the function has {} instructions, so there is no instruction {index}",
            decoded.len()
        );
    }
    let mut info = InstructionInfoFactory::new();
    let mut renamed = Vec::with_capacity(code.len());
    let mut points = Vec::with_capacity(decoded.len());
    for (index, instruction) in decoded.iter().enumerate() {
        let (at, end) = (starts[index], starts[index] + instruction.len());
        ensure!(
            !instruction.is_invalid(),
            "cannot decode the instruction at {at:#x}"
        );
        let rho = map.get(&index).cloned().unwrap_or_default();
        let (reads, writes) = footprint(&mut info, instruction);
        let bytes = encode(instruction, &rho, &reads, &writes, &mut info)
            .with_context(|| format!("cannot rename the instruction at {at:#x}"))?;
        let fields: Vec<usize> = relocations
            .iter()
            .copied()
            .filter(|field| (at..end).contains(field))
            .collect();
        for &field in &fields {
            let width = 4.min(end - field);
            ensure!(
                bytes[field - at..field - at + width] == code[field..field + width],
                "renaming changes the relocated field in the instruction at {at:#x}"
            );
        }
        renamed.extend(bytes);
        let (next, exit) = successors(
            instruction,
            index,
            &starts,
            decoded.len(),
            !fields.is_empty(),
        )?;
        points.push(Point {
            reads,
            writes,
            rho,
            next,
            exit,
        });
    }
    check(&points, &starts)?;
    Ok(renamed)
}

/// Where control goes after instruction `index`, and what must hold if it
/// leaves the function. A relocated branch leaves it.
fn successors(
    instruction: &Instruction,
    index: usize,
    starts: &[usize],
    count: usize,
    relocated: bool,
) -> Result<(Vec<usize>, Vec<Register>)> {
    let target = usize::try_from(instruction.near_branch_target())
        .ok()
        .filter(|_| !relocated)
        .and_then(|target| starts.iter().position(|&start| start == target));
    let following = (index + 1 < count).then_some(index + 1);
    Ok(match instruction.flow_control() {
        FlowControl::Next | FlowControl::Call | FlowControl::IndirectCall => match following {
            Some(next) => (vec![next], Vec::new()),
            None => (Vec::new(), GENERAL.to_vec()),
        },
        FlowControl::Return => (Vec::new(), RETURNED.to_vec()),
        FlowControl::ConditionalBranch => match (target, following) {
            (Some(target), Some(next)) => (vec![target, next], Vec::new()),
            _ => bail!(
                "the branch at {:#x} leaves the function, which a rename does not follow",
                starts[index]
            ),
        },
        // A jump out of the function is a tail call.
        FlowControl::UnconditionalBranch => match target {
            Some(target) => (vec![target], Vec::new()),
            None => (Vec::new(), GENERAL.to_vec()),
        },
        FlowControl::IndirectBranch => bail!(
            "the function jumps through a table at {:#x}, so data may follow its code",
            starts[index]
        ),
        _ => (Vec::new(), GENERAL.to_vec()),
    })
}

/// Re-encodes `instruction` with its operands' registers renamed by `rho`,
/// and checks that its footprint is exactly `rho` of the original's, so it
/// is the same operation on renamed registers (Lean `Renamed.Sound`).
fn encode(
    instruction: &Instruction,
    rho: &BTreeMap<Register, Register>,
    reads: &BTreeSet<Register>,
    writes: &BTreeSet<Register>,
    info: &mut InstructionInfoFactory,
) -> Result<Vec<u8>> {
    let named = explicit(instruction);
    for from in rho.keys() {
        ensure!(
            *from != Register::ESP && rho[from] != Register::ESP,
            "esp is the stack pointer and is never renamed"
        );
        ensure!(named.contains(from), "{from:?} is not named by an operand");
    }
    let image = |register: Register| rho.get(&register).copied().unwrap_or(register);
    let used: BTreeSet<Register> = reads.union(writes).chain(&named).copied().collect();
    let images: BTreeSet<Register> = used.iter().map(|&register| image(register)).collect();
    ensure!(
        images.len() == used.len(),
        "two of its registers would become one"
    );
    let swap = |register: Register| match rho.get(&register.full_register32()) {
        Some(&to) if register != Register::None => counterpart(register, to),
        _ => Ok(register),
    };
    let mut changed = *instruction;
    for operand in 0..instruction.op_count() {
        match instruction.op_kind(operand) {
            OpKind::Register => {
                changed.set_op_register(operand, swap(instruction.op_register(operand))?);
            }
            OpKind::Memory => {
                changed.set_memory_base(swap(instruction.memory_base())?);
                changed.set_memory_index(swap(instruction.memory_index())?);
            }
            _ => {}
        }
    }
    let mut encoder = Encoder::new(32);
    encoder.encode(&changed, instruction.ip())?;
    let bytes = encoder.take_buffer();
    ensure!(
        bytes.len() == instruction.len(),
        "its encoding would change length"
    );
    let redecoded = Decoder::with_ip(32, &bytes, instruction.ip(), DecoderOptions::NONE).decode();
    let (new_reads, new_writes) = footprint(info, &redecoded);
    ensure!(
        new_reads == reads.iter().map(|&register| image(register)).collect()
            && new_writes == writes.iter().map(|&register| image(register)).collect(),
        "the renamed instruction uses registers its operands do not name"
    );
    Ok(bytes)
}

/// Lean `Checked`: computes `σ` and checks reads and exits.
fn check(points: &[Point], starts: &[usize]) -> Result<()> {
    if points.is_empty() {
        return Ok(());
    }
    let mut sigma: Vec<Option<BTreeSet<(Register, Register)>>> = vec![None; points.len()];
    sigma[0] = Some(
        GENERAL
            .iter()
            .map(|&register| (register, register))
            .collect(),
    );
    let mut work = vec![0];
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
        if let Some(&read) = point
            .reads
            .iter()
            .find(|&&read| !current.contains(&(point.rho(read), read)))
        {
            bail!(
                "the instruction at {:#x} would read {:?} where the original read {read:?}, which may hold another value there",
                starts[index],
                point.rho(read)
            );
        }
        let after = point.post(current);
        if let Some(register) = point
            .exit
            .iter()
            .find(|&&register| !after.contains(&(register, register)))
        {
            bail!(
                "{register:?} would hold another value when the function leaves at {:#x}",
                starts[index]
            );
        }
    }
    Ok(())
}
