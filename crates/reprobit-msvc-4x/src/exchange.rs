//! `exchange`: rewrites a short run of instructions into another run that
//! computes the same values, where the old compiler picked the other of two
//! equal forms:
//!
//! - `fld X; fadd Y` or `fmul Y` becomes `fld Y; fadd X`, since addition and
//!   multiplication commute;
//! - `cmp a, b; jcc` becomes `cmp b, a` with the mirrored branch, such as
//!   `jl` for `jg`;
//! - `mov r, [A]; cmp [B], r; je` becomes `mov r, [B]; cmp [A], r`, since
//!   equality is symmetric.
//!
//! The mirrored forms set every flag but `ZF` differently, and the slot
//! exchange leaves another value in `r`: those are the run's clobbers, which
//! `correspond` checks nothing reads.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use iced_x86::{
    Code, ConditionCode, Encoder, FlowControl, Instruction, InstructionInfoFactory, OpKind,
    RflagsBits,
};

use crate::coff::Function;
use crate::correspond::{self, Point, footprint};
use crate::schedule::Location;

/// One run to rewrite, by its first instruction's index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Site {
    pub(crate) index: usize,
    /// For a compare of two registers, swap its `ModRM` fields rather than
    /// flip its opcode's direction.
    pub(crate) fields: bool,
}

/// Parses `i,j:fields,…`.
pub(crate) fn parse(text: &str) -> Result<Vec<Site>> {
    text.split(',')
        .map(|item| {
            let (index, fields) = match item.trim().split_once(':') {
                Some((index, "fields")) => (index, true),
                Some(_) => bail!("{item} is not index or index:fields"),
                None => (item.trim(), false),
            };
            Ok(Site {
                index: index
                    .parse()
                    .with_context(|| format!("{index} is not an instruction index"))?,
                fields,
            })
        })
        .collect()
}

/// Writes `sites` as `parse` reads them.
pub(crate) fn format(sites: &[Site]) -> String {
    sites
        .iter()
        .map(|site| {
            if site.fields {
                format!("{}:fields", site.index)
            } else {
                site.index.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// The sites that turn `built` into `reference`, checked by applying them.
/// When `loose`, they turn `built` into code with `reference`'s instruction
/// forms, whatever registers it names, and each changes a form.
pub(crate) fn derive(built: &Function, reference: &[u8], loose: bool) -> Option<Vec<Site>> {
    let decoded = correspond::decode(&built.bytes).ok()?;
    if built.bytes.len() != reference.len() {
        return None;
    }
    let alike = |ours: &[u8], theirs: &[u8]| {
        let forms = |code: &[u8]| -> Option<Vec<String>> {
            Some(
                correspond::decode(code)
                    .ok()?
                    .iter()
                    .map(correspond::form)
                    .collect(),
            )
        };
        forms(ours).is_some_and(|forms_ours| Some(forms_ours) == forms(theirs))
    };
    let mut sites = Vec::new();
    let mut free = 0;
    for index in 0..decoded.len() {
        for fields in [false, true] {
            let site = Site { index, fields };
            let Ok((last, bytes, _)) = rewrite(&decoded, &built.bytes, site) else {
                continue;
            };
            let (start, end) = span(&decoded, index, last);
            let wanted = &reference[start..end];
            let matches = if loose {
                alike(&bytes, wanted) && !alike(&built.bytes[start..end], wanted)
            } else {
                bytes == wanted
            };
            if index >= free && matches {
                sites.push(site);
                free = last + 1;
                break;
            }
        }
    }
    if sites.is_empty() {
        return None;
    }
    let exchanged = apply(built, &sites).ok()?;
    (loose || exchanged == reference).then_some(sites)
}

/// The byte range instructions `first..=last` occupy.
fn span(decoded: &[Instruction], first: usize, last: usize) -> (usize, usize) {
    let start = usize::try_from(decoded[first].ip()).unwrap_or_default();
    let end = usize::try_from(decoded[last].next_ip()).unwrap_or_default();
    (start, end)
}

/// Rewrites the runs at `sites` in `function`, after checking the result
/// computes what the original did.
pub(crate) fn apply(function: &Function, sites: &[Site]) -> Result<Vec<u8>> {
    let decoded = correspond::decode(&function.bytes)?;
    let fields = function.fields();
    let mut info = InstructionInfoFactory::new();
    let mut points = decoded
        .iter()
        .enumerate()
        .map(|(index, instruction)| {
            let relocated = !correspond::fields(instruction, &fields).is_empty();
            Point::new(&mut info, &decoded, index, relocated, BTreeMap::new())
        })
        .collect::<Result<Vec<_>>>()?;
    let targets: BTreeSet<u64> = decoded
        .iter()
        .filter(|instruction| {
            matches!(
                instruction.flow_control(),
                FlowControl::ConditionalBranch | FlowControl::UnconditionalBranch
            )
        })
        .map(Instruction::near_branch_target)
        .collect();
    let mut code = function.bytes.clone();
    let mut free = 0;
    for &site in sites {
        ensure!(
            site.index >= free && site.index < decoded.len(),
            "the sites must name distinct runs in order, within the function's {} instructions",
            decoded.len()
        );
        let (last, bytes, clobbers) = rewrite(&decoded, &function.bytes, site)?;
        let (start, end) = span(&decoded, site.index, last);
        for inner in &decoded[site.index + 1..=last] {
            let at = inner.ip();
            let offset = usize::try_from(at)?;
            ensure!(
                !targets.contains(&at)
                    && !function.barriers.contains(&offset)
                    && !function.entries.contains(&offset),
                "control reaches {at:#x} other than from the instruction before it, so the run there cannot change"
            );
        }
        for field in fields
            .iter()
            .filter(|&&field| (start..end).contains(&field))
        {
            let width = 4.min(end - field);
            ensure!(
                bytes[field - start..field - start + width] == code[*field..field + width],
                "the exchange at {start:#x} would change or move a relocated field"
            );
        }
        same_footprint(&mut info, &decoded[site.index..=last], &bytes, start)?;
        code[start..end].copy_from_slice(&bytes);
        points[site.index] = Point::run(&points, site.index, last, &clobbers);
        free = last + 1;
    }
    correspond::check(&points, &decoded, &correspond::entries(&decoded, function)?)?;
    Ok(code)
}

/// Checks the new run reads and writes the registers and flags the old one did.
fn same_footprint(
    info: &mut InstructionInfoFactory,
    old: &[Instruction],
    bytes: &[u8],
    start: usize,
) -> Result<()> {
    let new = correspond::decode(bytes)?;
    let union = |info: &mut InstructionInfoFactory, run: &[Instruction]| {
        run.iter().fold(
            (BTreeSet::new(), BTreeSet::new()),
            |(mut reads, mut writes): (BTreeSet<Location>, BTreeSet<Location>), instruction| {
                let (read, write) = footprint(info, instruction);
                reads.extend(read.difference(&writes));
                writes.extend(write);
                (reads, writes)
            },
        )
    };
    ensure!(
        new.len() == old.len() && union(info, &new) == union(info, old),
        "the exchange at {start:#x} would use other registers or flags"
    );
    Ok(())
}

/// The new bytes for the run at `site`, its last instruction, and its
/// clobbers, from the run's shape alone.
fn rewrite(
    decoded: &[Instruction],
    code: &[u8],
    site: Site,
) -> Result<(usize, Vec<u8>, BTreeSet<Location>)> {
    let first = decoded
        .get(site.index)
        .with_context(|| format!("the function has no instruction {}", site.index))?;
    let next = decoded.get(site.index + 1);
    match first.code() {
        Code::Fld_m32fp | Code::Fld_m64fp => operands(decoded, site.index),
        Code::Mov_r32_rm32 if next.is_some_and(|next| next.code() == Code::Cmp_rm32_r32) => {
            slots(decoded, code, site.index)
        }
        Code::Cmp_rm8_r8
        | Code::Cmp_rm16_r16
        | Code::Cmp_rm32_r32
        | Code::Cmp_r8_rm8
        | Code::Cmp_r16_rm16
        | Code::Cmp_r32_rm32 => mirror(decoded, code, site),
        _ => bail!(
            "the instruction at {:#x} starts no exchange: an x87 load and operation, a compare and branch, or a load, compare, and equality branch",
            first.ip()
        ),
    }
}

/// Encodes `instructions` one after another from the first's address.
fn encode(instructions: &[Instruction]) -> Result<Vec<u8>> {
    let mut encoder = Encoder::new(32);
    let mut ip = instructions.first().map_or(0, Instruction::ip);
    for instruction in instructions {
        ip += u64::try_from(encoder.encode(instruction, ip)?)?;
    }
    Ok(encoder.take_buffer())
}

/// `instruction` addressing `from`'s memory operand.
fn with_memory(instruction: &Instruction, from: &Instruction) -> Instruction {
    let mut changed = *instruction;
    changed.set_memory_base(from.memory_base());
    changed.set_memory_index(from.memory_index());
    changed.set_memory_index_scale(from.memory_index_scale());
    changed.set_memory_displacement32(from.memory_displacement32());
    changed.set_memory_displ_size(from.memory_displ_size());
    changed.set_segment_prefix(from.segment_prefix());
    changed
}

/// `fld X; fadd Y` or `fmul Y` as `fld Y; fadd X` or `fmul X`.
fn operands(decoded: &[Instruction], index: usize) -> Result<(usize, Vec<u8>, BTreeSet<Location>)> {
    let load = &decoded[index];
    let operation = decoded
        .get(index + 1)
        .context("the x87 load ends the function")?;
    let pairs = match load.code() {
        Code::Fld_m32fp => [Code::Fadd_m32fp, Code::Fmul_m32fp],
        _ => [Code::Fadd_m64fp, Code::Fmul_m64fp],
    };
    ensure!(
        pairs.contains(&operation.code()),
        "the x87 load at {:#x} is not followed by an addition or multiplication of the same width",
        load.ip()
    );
    let bytes = encode(&[with_memory(load, operation), with_memory(operation, load)])?;
    let old = encode(&[*load, *operation])?;
    ensure!(
        bytes != old,
        "the x87 load and operation at {:#x} already read the same memory",
        load.ip()
    );
    ensure!(
        bytes.len() == old.len(),
        "the exchanged pair at {:#x} would change length",
        load.ip()
    );
    Ok((index + 1, bytes, BTreeSet::new()))
}

/// The flags a compare sets other than `ZF`.
fn unequal_flags(compare: &Instruction) -> BTreeSet<Location> {
    correspond::flags(compare.rflags_modified() & !RflagsBits::ZF).collect()
}

/// `cmp a, b; jcc` as `cmp b, a` and the mirrored branch.
fn mirror(
    decoded: &[Instruction],
    code: &[u8],
    site: Site,
) -> Result<(usize, Vec<u8>, BTreeSet<Location>)> {
    let compare = &decoded[site.index];
    let branch = decoded
        .get(site.index + 1)
        .filter(|branch| branch.flow_control() == FlowControl::ConditionalBranch)
        .with_context(|| {
            format!(
                "the compare at {:#x} is not followed by a conditional branch",
                compare.ip()
            )
        })?;
    let (start, end) = span(decoded, site.index, site.index + 1);
    let mut bytes = code[start..end].to_vec();
    let opcode = bytes
        .iter()
        .position(|byte| ![0x26, 0x2e, 0x36, 0x3e, 0x64, 0x65, 0x66].contains(byte))
        .context("the compare is only prefixes")?;
    if site.fields {
        let modrm = bytes[opcode + 1];
        ensure!(
            modrm >> 6 == 3,
            "the compare at {:#x} reads memory, so only its opcode can flip",
            compare.ip()
        );
        bytes[opcode + 1] = 0xc0 | (modrm & 7) << 3 | (modrm >> 3 & 7);
    } else {
        bytes[opcode] ^= 2;
    }
    let at = compare.len() + usize::from(bytes[compare.len()] == 0x0f);
    ensure!(
        matches!(bytes[at] & 0xf0, 0x70 | 0x80),
        "the branch at {:#x} carries a prefix",
        branch.ip()
    );
    let condition = bytes[at] & 0x0f;
    let mirrored = match condition {
        // b, ae, e, ne, be, a; l, ge, le, g.
        0x2 => 0x7,
        0x7 => 0x2,
        0x3 => 0x6,
        0x6 => 0x3,
        0x4 | 0x5 => condition,
        0xc => 0xf,
        0xf => 0xc,
        0xd => 0xe,
        0xe => 0xd,
        _ => bail!(
            "the branch at {:#x} tests a flag whose mirror no condition reads",
            branch.ip()
        ),
    };
    bytes[at] = bytes[at] & 0xf0 | mirrored;
    let new = correspond::decode_at(&bytes[..compare.len()], compare.ip())?;
    ensure!(
        new.mnemonic() == compare.mnemonic()
            && same_operand(&new, 0, compare, 1)
            && same_operand(&new, 1, compare, 0),
        "the compare at {:#x} does not re-decode with its operands exchanged",
        compare.ip()
    );
    Ok((site.index + 1, bytes, unequal_flags(compare)))
}

/// Whether operand `i` of `a` is operand `j` of `b`.
fn same_operand(a: &Instruction, i: u32, b: &Instruction, j: u32) -> bool {
    a.op_kind(i) == b.op_kind(j)
        && match a.op_kind(i) {
            OpKind::Register => a.op_register(i) == b.op_register(j),
            OpKind::Memory => {
                a.memory_base() == b.memory_base()
                    && a.memory_index() == b.memory_index()
                    && a.memory_index_scale() == b.memory_index_scale()
                    && a.memory_displacement32() == b.memory_displacement32()
                    && a.memory_segment() == b.memory_segment()
            }
            _ => false,
        }
}

/// `mov r, [A]; cmp [B], r; je` as `mov r, [B]; cmp [A], r; je`.
fn slots(
    decoded: &[Instruction],
    code: &[u8],
    index: usize,
) -> Result<(usize, Vec<u8>, BTreeSet<Location>)> {
    let (load, compare) = (&decoded[index], &decoded[index + 1]);
    let branch = decoded.get(index + 2);
    let register = load.op0_register();
    ensure!(
        load.op1_kind() == OpKind::Memory
            && compare.op0_kind() == OpKind::Memory
            && compare.op1_register() == register,
        "the compare at {:#x} does not test memory against the register loaded before it",
        compare.ip()
    );
    ensure!(
        branch.is_some_and(|branch| {
            branch.flow_control() == FlowControl::ConditionalBranch
                && matches!(
                    branch.condition_code(),
                    ConditionCode::e | ConditionCode::ne
                )
        }),
        "the compare at {:#x} is not followed by je or jne",
        compare.ip()
    );
    let mut differ = *compare;
    differ.set_memory_displacement32(load.memory_displacement32());
    differ.set_memory_displ_size(load.memory_displ_size());
    ensure!(
        same_operand(&differ, 0, load, 1)
            && load.memory_displacement32() != compare.memory_displacement32()
            && load.memory_displ_size() == compare.memory_displ_size()
            && load.memory_base().full_register32() != register
            && load.memory_index().full_register32() != register,
        "the load at {:#x} and the compare after it do not address two slots off the same unchanged registers",
        load.ip()
    );
    let mut new_load = *load;
    new_load.set_memory_displacement32(compare.memory_displacement32());
    let mut new_compare = *compare;
    new_compare.set_memory_displacement32(load.memory_displacement32());
    let mut bytes = encode(&[new_load, new_compare])?;
    let (start, end) = span(decoded, index, index + 2);
    ensure!(
        bytes.len() == usize::try_from(compare.next_ip())? - start,
        "the exchanged slots at {:#x} would change length",
        load.ip()
    );
    bytes.extend(&code[usize::try_from(compare.next_ip())?..end]);
    let mut clobbers = unequal_flags(compare);
    clobbers.insert(Location::Register(register));
    Ok((index + 2, bytes, clobbers))
}
