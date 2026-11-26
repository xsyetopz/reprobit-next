//! `rename`: renames registers in chosen instructions, such as one value's
//! web or a stretch where two registers trade places. Each renamed
//! instruction must be the same operation on the renamed registers, and the
//! renamed function must pass the correspondence check in `correspond`.
//! A renamed memory base can change an instruction's length, as `ebp`
//! needs a displacement byte that other bases do not; `layout` writes such
//! a function back.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use iced_x86::{Code, Encoder, Instruction, InstructionInfoFactory, OpKind, Register};

use crate::coff::{Coff, Function};
use crate::correspond::{self, GENERAL, Point, explicit, footprint};
use crate::layout;
use crate::schedule::Location;

/// Per instruction index, a rename of 32-bit registers.
pub(crate) type Map = BTreeMap<usize, BTreeMap<Register, Register>>;

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

/// The rename that turns `symbol`, `built` in `object`, into `reference`
/// when they differ only in the registers their operands name and the
/// padding after them, checked by applying it.
pub(crate) fn derive(
    object: &[u8],
    symbol: &str,
    built: &Function,
    reference: &[u8],
) -> Option<Map> {
    let code = |bytes: &[u8]| -> Option<Vec<Instruction>> {
        let mut decoded = correspond::decode(bytes).ok()?;
        while decoded
            .last()
            .is_some_and(|last| matches!(last.code(), Code::Int3 | Code::Nopd))
        {
            decoded.pop();
        }
        Some(decoded)
    };
    let (ours, theirs) = (code(&built.bytes)?, code(reference)?);
    if ours.len() != theirs.len() {
        return None;
    }
    let mut map = Map::new();
    for (index, (our, their)) in ours.iter().zip(&theirs).enumerate() {
        let renames = between(our, their)?;
        if !renames.is_empty() {
            map.insert(index, renames);
        }
    }
    let renamed = apply(object, built, &map).ok()?;
    let renamed = Coff::parse(&renamed).ok()?.function(symbol).ok()?.bytes;
    (!map.is_empty() && renamed == reference).then_some(map)
}

/// The rename of `our`'s operand registers that gives `their`'s, when both
/// are the same operation.
pub(crate) fn between(
    our: &Instruction,
    their: &Instruction,
) -> Option<BTreeMap<Register, Register>> {
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
    if our.mnemonic() != their.mnemonic() {
        return None;
    }
    let mut renames = BTreeMap::new();
    for (from, to) in operands(our).into_iter().zip(operands(their)) {
        if from != to && *renames.entry(from).or_insert(to) != to {
            return None;
        }
    }
    Some(renames)
}

/// `instruction` with the registers its operands name renamed by `rho`.
pub(crate) fn renamed(
    instruction: &Instruction,
    rho: &BTreeMap<Register, Register>,
) -> Result<Instruction> {
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
    Ok(changed)
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

/// The object `object` with the registers `map` names renamed in
/// `function`, after checking the renamed code computes what the original
/// did.
pub(crate) fn apply(object: &[u8], function: &Function, map: &Map) -> Result<Vec<u8>> {
    let decoded = correspond::decode(&function.bytes)?;
    if let Some(index) = map.keys().find(|&&index| index >= decoded.len()) {
        bail!(
            "the function has {} instructions, so there is no instruction {index}",
            decoded.len()
        );
    }
    let fields = function.fields();
    let mut info = InstructionInfoFactory::new();
    let mut renamed = Vec::with_capacity(decoded.len());
    let mut points = Vec::with_capacity(decoded.len());
    for (index, instruction) in decoded.iter().enumerate() {
        let rho = map.get(&index).cloned().unwrap_or_default();
        let relocated = !correspond::fields(instruction, &fields).is_empty();
        let point = Point::new(&mut info, &decoded, index, relocated, rho)?;
        let at = instruction.ip();
        renamed.push(
            encode(instruction, &point, &mut info)
                .with_context(|| format!("cannot rename the instruction at {at:#x}"))?,
        );
        points.push(point);
    }
    correspond::check(&points, &decoded, &correspond::entries(&decoded, function)?)?;
    layout::lay_out(object, function, &decoded, renamed)
}

/// Re-encodes `instruction` with its operands' registers renamed by the
/// point's `rho`, and checks that its footprint is exactly `rho` of the
/// original's, so it is the same operation on renamed registers (Lean
/// `Renamed.Sound`).
fn encode(
    instruction: &Instruction,
    point: &Point,
    info: &mut InstructionInfoFactory,
) -> Result<Vec<u8>> {
    let rho = &point.rho;
    let named = explicit(instruction);
    for from in rho.keys() {
        ensure!(
            *from != Register::ESP && rho[from] != Register::ESP,
            "esp is the stack pointer and is never renamed"
        );
        ensure!(named.contains(from), "{from:?} is not named by an operand");
    }
    let image = |location: Location| match location {
        Location::Register(register) => {
            Location::Register(rho.get(&register).copied().unwrap_or(register))
        }
        other => other,
    };
    let used: BTreeSet<Location> = point
        .reads
        .union(&point.writes)
        .copied()
        .chain(named.iter().map(|&register| Location::Register(register)))
        .collect();
    let images: BTreeSet<Location> = used.iter().map(|&location| image(location)).collect();
    ensure!(
        images.len() == used.len(),
        "two of its registers would become one"
    );
    let changed = renamed(instruction, rho)?;
    let mut encoder = Encoder::new(32);
    encoder.encode(&changed, instruction.ip())?;
    let bytes = encoder.take_buffer();
    let redecoded = correspond::decode_at(&bytes, instruction.ip())?;
    let (reads, writes) = footprint(info, &redecoded);
    let mapped = |set: &BTreeSet<Location>| -> BTreeSet<Location> {
        set.iter().map(|&location| image(location)).collect()
    };
    ensure!(
        reads == mapped(&point.reads) && writes == mapped(&point.writes),
        "the renamed instruction uses registers its operands do not name"
    );
    Ok(bytes)
}
