//! `region`: reorders a straight run of instructions and renames registers
//! in them at once, where neither alone keeps the values: `mov eax, a;
//! mov ecx, b; add eax, ecx` and `mov ecx, a; mov eax, b; add eax, ecx`
//! leave the same sum in `eax`. `simulate` executes both runs from one
//! start state. The registers and flags they leave with different values
//! are the run's clobbers, which `correspond` checks nothing reads (Lean
//! Theorem 3), and memory and the x87 state must end the same.

mod simulate;

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use iced_x86::{Encoder, FlowControl, Instruction, InstructionInfoFactory, Register};

use crate::coff::{Coff, Function};
use crate::correspond::{self, GENERAL, Point, footprint};
use crate::rename::{self, Map};
use crate::schedule::Location;
use simulate::{State, Step, simulate};

/// A run to rewrite: the order of its instructions, by index, and the
/// renames in them, by their original index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Region {
    pub(crate) order: Vec<usize>,
    pub(crate) map: Map,
}

/// Parses `i,j,…[/i:from:to,…]`.
pub(crate) fn parse(text: &str) -> Result<Region> {
    let (order, map) = text.split_once('/').unwrap_or((text, ""));
    Ok(Region {
        order: order
            .split(',')
            .map(|item| {
                item.trim()
                    .parse()
                    .with_context(|| format!("{item} is not an instruction index"))
            })
            .collect::<Result<_>>()?,
        map: if map.is_empty() {
            Map::new()
        } else {
            rename::parse(map)?
        },
    })
}

/// Writes `region` as `parse` reads it.
pub(crate) fn format(region: &Region) -> String {
    let order: Vec<String> = region.order.iter().map(ToString::to_string).collect();
    match rename::format(&region.map) {
        map if map.is_empty() => order.join(","),
        map => format!("{}/{map}", order.join(",")),
    }
}

/// The region that turns `symbol`, `built` in `object`, into `reference`:
/// the span between their common first and last instructions, with each
/// of reference's instructions taken from the first unused one of the
/// same form and displacement, checked by applying it. The run grows over
/// the common instructions after it until the values it leaves different
/// are dead, as when a sum reads both renamed operands.
pub(crate) fn derive(
    object: &[u8],
    symbol: &str,
    built: &Function,
    reference: &[u8],
) -> Option<Region> {
    let ours = correspond::decode(&built.bytes).ok()?;
    let theirs = correspond::decode(reference).ok()?;
    if ours.len() != theirs.len() || built.bytes.len() != reference.len() {
        return None;
    }
    let bytes = |code: &[u8], instruction: &Instruction| {
        let start = usize::try_from(instruction.ip()).unwrap_or_default();
        code.get(start..start + instruction.len())
            .map(<[u8]>::to_vec)
    };
    let same =
        |index: &usize| bytes(&built.bytes, &ours[*index]) == bytes(reference, &theirs[*index]);
    let first = (0..ours.len()).find(|index| !same(index))?;
    let last = (first..ours.len()).rev().find(|index| !same(index))?;
    // A run is not rebased across pushes, so `[esp+d]` keeps its `d`.
    let key = |instruction: &Instruction| {
        (
            correspond::form(instruction),
            instruction.memory_displacement32(),
        )
    };
    let mut used = vec![false; ours.len()];
    let mut region = Region {
        order: Vec::new(),
        map: Map::new(),
    };
    for their in &theirs[first..=last] {
        let index =
            (first..=last).find(|&index| !used[index] && key(&ours[index]) == key(their))?;
        used[index] = true;
        region.order.push(index);
        let renames = rename::between(&ours[index], their)?;
        if !renames.is_empty() {
            region.map.insert(index, renames);
        }
    }
    (last..ours.len())
        .take_while(|&end| end == last || ours[end].flow_control() == FlowControl::Next)
        .find_map(|end| {
            let mut grown = region.clone();
            grown.order.extend(last + 1..=end);
            let rewritten = apply(object, built, &grown).ok()?;
            let function = Coff::parse(&rewritten).ok()?.function(symbol).ok()?;
            (function.bytes == reference).then_some(grown)
        })
}

/// The object `object` with `region` rewritten in `function`, after
/// checking the new run computes what the old one did.
pub(crate) fn apply(object: &[u8], function: &Function, region: &Region) -> Result<Vec<u8>> {
    let decoded = correspond::decode(&function.bytes)?;
    let (first, last) = run(function, &decoded, region)?;
    let start = usize::try_from(decoded[first].ip())?;
    let mut fields = BTreeMap::new();
    let (code, new) = emit(function, &decoded, region, start, &mut fields)?;
    let old = decoded[first..=last]
        .iter()
        .map(|instruction| Step::new(*instruction, symbols(function, instruction)))
        .collect::<Result<Vec<_>>>()?;
    let (before, after) = (simulate(&old)?, simulate(&new)?);
    prove(function, &decoded, (first, last), (&before, &after), &new)?;
    let place: BTreeMap<usize, usize> = decoded
        .iter()
        .filter_map(|instruction| usize::try_from(instruction.ip()).ok())
        .chain([function.bytes.len()])
        .map(|offset| (offset, offset))
        .collect();
    for field in function.fields() {
        fields.entry(field).or_insert(field);
    }
    let whole = [
        &function.bytes[..start],
        &code,
        &function.bytes[start + code.len()..],
    ]
    .concat();
    Coff::parse(object)?.replace(function, &whole, &place, &fields)
}

/// The first and last instruction of the run `region` reorders, after
/// checking that it is one straight run entered only at its start.
fn run(function: &Function, decoded: &[Instruction], region: &Region) -> Result<(usize, usize)> {
    let (first, last) = match (region.order.iter().min(), region.order.iter().max()) {
        (Some(&first), Some(&last)) if last < decoded.len() => (first, last),
        _ => bail!(
            "the order names no instruction of the function's {}",
            decoded.len()
        ),
    };
    let mut sorted = region.order.clone();
    sorted.sort_unstable();
    ensure!(
        sorted.iter().copied().eq(first..=last),
        "the order is not a permutation of one run of instructions"
    );
    ensure!(
        region
            .map
            .keys()
            .all(|index| (first..=last).contains(index)),
        "a rename lies outside the run"
    );
    let targets: BTreeSet<u64> = decoded
        .iter()
        .filter(|instruction| instruction.flow_control() != FlowControl::Next)
        .map(Instruction::near_branch_target)
        .collect();
    for (index, instruction) in decoded.iter().enumerate().take(last + 1).skip(first) {
        let at = instruction.ip();
        ensure!(
            instruction.flow_control() == FlowControl::Next,
            "the instruction at {at:#x} transfers control, so the run is not straight"
        );
        let offset = usize::try_from(at)?;
        ensure!(
            index == first
                || !(targets.contains(&at)
                    || function.barriers.contains(&offset)
                    || function.entries.contains(&offset)),
            "control reaches {at:#x} other than from the instruction before it, so the run there cannot change"
        );
    }
    Ok((first, last))
}

/// The run's new code from `start` and each new instruction to simulate,
/// and, into `fields`, where each relocated field moves.
fn emit(
    function: &Function,
    decoded: &[Instruction],
    region: &Region,
    start: usize,
    fields: &mut BTreeMap<usize, usize>,
) -> Result<(Vec<u8>, Vec<Step>)> {
    let (mut code, mut new) = (Vec::new(), Vec::new());
    for &index in &region.order {
        let original = &decoded[index];
        let rho = region.map.get(&index).cloned().unwrap_or_default();
        ensure!(
            !rho.contains_key(&Register::ESP) && !rho.values().any(|&to| to == Register::ESP),
            "esp is the stack pointer and is never renamed"
        );
        let ip = u64::try_from(start + code.len())?;
        let mut encoder = Encoder::new(32);
        encoder.encode(&rename::renamed(original, &rho)?, ip)?;
        let bytes = encoder.take_buffer();
        let from = usize::try_from(original.ip())?;
        ensure!(
            bytes.len() == original.len(),
            "renaming the instruction at {from:#x} would change its length"
        );
        for field in correspond::fields(original, &function.fields()) {
            let within = field - from;
            ensure!(
                bytes.get(within..within + 4) == function.bytes.get(field..field + 4),
                "renaming changes the relocated field in the instruction at {from:#x}"
            );
            fields.insert(field, start + code.len() + within);
        }
        let instruction = correspond::decode_at(&bytes, ip)?;
        new.push(Step::new(instruction, symbols(function, original))?);
        code.extend(bytes);
    }
    Ok((code, new))
}

/// Checks the new run against the old: memory and the x87 state end the
/// same, and the function passes the correspondence check with the run as
/// one point whose clobbers are the locations the runs leave different.
fn prove(
    function: &Function,
    decoded: &[Instruction],
    (first, last): (usize, usize),
    (before, after): (&State, &State),
    new: &[Step],
) -> Result<()> {
    ensure!(
        before.memory() == after.memory(),
        "the two runs leave memory different"
    );
    ensure!(
        before.value(Location::Float) == after.value(Location::Float),
        "the two runs leave the x87 state different"
    );
    let mut info = InstructionInfoFactory::new();
    let relocated = function.fields();
    let mut points = decoded
        .iter()
        .enumerate()
        .map(|(index, instruction)| {
            let inside = !correspond::fields(instruction, &relocated).is_empty();
            Point::new(&mut info, decoded, index, inside, BTreeMap::new())
        })
        .collect::<Result<Vec<_>>>()?;
    let clobbers: BTreeSet<Location> = GENERAL
        .into_iter()
        .map(Location::Register)
        .chain(correspond::flags(u32::MAX))
        .filter(|&location| before.value(location) != after.value(location))
        .collect();
    let mut point = Point::run(&points, first, last, &clobbers);
    let mut written = BTreeSet::new();
    for step in new {
        let (reads, writes) = footprint(&mut info, step.instruction());
        point.reads.extend(reads.difference(&written).copied());
        written.extend(writes);
    }
    point
        .writes
        .extend(written.into_iter().chain(clobbers.iter().copied()));
    point.kept = point.writes.difference(&clobbers).copied().collect();
    points[first] = point;
    correspond::check(&points, decoded, &correspond::entries(decoded, function)?)
}

/// The relocation targets of `instruction`'s fields, by offset within it.
fn symbols(function: &Function, instruction: &Instruction) -> BTreeMap<usize, String> {
    let start = usize::try_from(instruction.ip()).unwrap_or_default();
    function
        .relocations
        .iter()
        .filter(|relocation| (start..start + instruction.len()).contains(&relocation.offset))
        .map(|relocation| (relocation.offset - start, relocation.target.clone()))
        .collect()
}
