//! `rbit-msvc-4x`, a `ReproBit` add-on for i386 COFF objects from MSVC 4.x.
//! Its rewrites change what a newer source cannot steer the old compiler
//! to: `schedule` reorders a function's instructions to a declared order
//! that keeps every dependence, and `rename` renames registers in chosen
//! instructions where every value still reaches the same uses.
//!
//! - `rbit-msvc-4x rewrite <object> schedule --symbol <name> --order <i,j,…>`
//! - `rbit-msvc-4x rewrite <object> rename --symbol <name> --map <i:from:to,…>`
//! - `rbit-msvc-4x propose <built> <reference> <name>`

mod coff;
mod rename;
mod schedule;

use std::io::Write as _;
use std::process::ExitCode;
use std::{env, fs};

use anyhow::{Context, Result, bail, ensure};

use crate::coff::{Coff, Function};
use crate::schedule::Instruction;

#[expect(
    clippy::print_stderr,
    reason = "the add-on protocol reads a failure's reason from stderr"
)]
fn main() -> ExitCode {
    match run(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rbit-msvc-4x: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<()> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [
            "rewrite",
            object,
            "schedule",
            "--symbol",
            symbol,
            "--order",
            order,
        ] => {
            let order = order
                .split(',')
                .map(|item| {
                    item.trim()
                        .parse::<usize>()
                        .with_context(|| format!("{item} is not an instruction index"))
                })
                .collect::<Result<Vec<_>>>()?;
            let mut bytes = fs::read(object).with_context(|| format!("cannot read {object}"))?;
            reorder(&mut bytes, symbol, &order)?;
            fs::write(object, bytes).with_context(|| format!("cannot write {object}"))
        }
        [
            "rewrite",
            object,
            "rename",
            "--symbol",
            symbol,
            "--map",
            map,
        ] => {
            let map = rename::parse(map)?;
            let mut bytes = fs::read(object).with_context(|| format!("cannot read {object}"))?;
            let function = Coff::parse(&bytes)?.function(symbol)?;
            let renamed = rename::apply(&function.bytes, &function.fields(), &map)?;
            bytes[function.file..function.file + renamed.len()].copy_from_slice(&renamed);
            fs::write(object, bytes).with_context(|| format!("cannot write {object}"))
        }
        ["propose", built, reference, symbol] => {
            let (built, reference) = (fs::read(built)?, fs::read(reference)?);
            let (ours, theirs) = (
                Coff::parse(&built)?.function(symbol)?,
                Coff::parse(&reference)?.function(symbol)?,
            );
            let order = propose(&built, &reference, symbol)?.map(|order| {
                let order: Vec<String> = order.iter().map(ToString::to_string).collect();
                ("schedule", "--order", order.join(","))
            });
            let map = rename::derive(&ours.bytes, &theirs.bytes, &ours.fields())
                .map(|map| ("rename", "--map", rename::format(&map)));
            let proposals: Vec<Vec<String>> = order
                .into_iter()
                .chain(map)
                .map(|(rewrite, flag, value)| {
                    [rewrite, "--symbol", symbol, flag, &value]
                        .map(str::to_owned)
                        .to_vec()
                })
                .collect();
            writeln!(std::io::stdout(), "{}", serde_json::to_string(&proposals)?)?;
            Ok(())
        }
        _ => bail!(
            "usage: rbit-msvc-4x rewrite <object> schedule --symbol <name> --order <i,j,…> | rbit-msvc-4x rewrite <object> rename --symbol <name> --map <i:from:to,…> | rbit-msvc-4x propose <built> <reference> <name>"
        ),
    }
}

fn decode(function: &Function) -> Result<Vec<Instruction>> {
    schedule::decode(&function.bytes, &function.barriers)
}

/// Writes `symbol`'s instructions in `order`, after checking it keeps
/// every dependence, and moves each relocation with its instruction.
fn reorder(bytes: &mut [u8], symbol: &str, order: &[usize]) -> Result<()> {
    let function = Coff::parse(bytes)?.function(symbol)?;
    let instructions = decode(&function)?;
    schedule::check(&instructions, order)?;
    let emitted = schedule::emit(&instructions, order)?;
    let mut code = Vec::with_capacity(function.bytes.len());
    let mut moved = Vec::new();
    for (&index, bytes) in order.iter().zip(emitted) {
        let instruction = &instructions[index];
        let range = instruction.start..instruction.start + instruction.bytes.len();
        for relocation in function
            .relocations
            .iter()
            .filter(|relocation| range.contains(&relocation.offset))
        {
            // A relocation entry holds a section offset.
            moved.push((
                relocation.entry,
                function.start + code.len() + relocation.offset - instruction.start,
            ));
        }
        code.extend(bytes);
    }
    ensure!(
        code.len() == function.bytes.len(),
        "the function has bytes after its last instruction"
    );
    bytes[function.file..function.file + code.len()].copy_from_slice(&code);
    for (entry, offset) in moved {
        bytes[entry..entry + 4].copy_from_slice(&u32::try_from(offset)?.to_le_bytes());
    }
    Ok(())
}

/// The order that turns `symbol` in `built` into the one in `reference`,
/// when both hold the same instructions with the same relocations, up to
/// the displacements that rebasing `[esp+d]` operands changes.
fn propose(built: &[u8], reference: &[u8], symbol: &str) -> Result<Option<Vec<usize>>> {
    let (built, reference) = (
        Coff::parse(built)?.function(symbol)?,
        Coff::parse(reference)?.function(symbol)?,
    );
    if built.bytes.len() != reference.bytes.len() {
        return Ok(None);
    }
    let (ours, theirs) = (decode(&built)?, decode(&reference)?);
    let key = |function: &Function, instruction: &Instruction| {
        let range = instruction.start..instruction.start + instruction.bytes.len();
        let relocations: Vec<_> = function
            .relocations
            .iter()
            .filter(|relocation| range.contains(&relocation.offset))
            .map(|relocation| {
                (
                    relocation.offset - instruction.start,
                    relocation.target.clone(),
                    relocation.kind,
                )
            })
            .collect();
        (instruction.shape(), instruction.slot(), relocations)
    };
    let mut used = vec![false; ours.len()];
    let mut order = Vec::with_capacity(ours.len());
    for wanted in &theirs {
        let wanted = key(&reference, wanted);
        let Some(index) =
            (0..ours.len()).find(|&index| !used[index] && key(&built, &ours[index]) == wanted)
        else {
            return Ok(None);
        };
        used[index] = true;
        order.push(index);
    }
    let identity = order.iter().copied().eq(0..order.len());
    Ok((!identity
        && order.len() == ours.len()
        && schedule::check(&ours, &order).is_ok()
        && schedule::emit(&ours, &order).is_ok())
    .then_some(order))
}
