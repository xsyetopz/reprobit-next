//! `rbit-msvc-4x`, a `ReproBit` add-on for i386 COFF objects from MSVC 4.x.
//! Its rewrites change what a newer source cannot steer the old compiler
//! to: `schedule` reorders a function's instructions to a declared order
//! that keeps every dependence, `rename` renames registers in chosen
//! instructions where every value still reaches the same uses, and
//! `exchange` swaps operands where the old compiler picked the other of two
//! equal forms.
//!
//! - `rbit-msvc-4x rewrite <object> schedule --symbol <name> --order <i,j,…>`
//! - `rbit-msvc-4x rewrite <object> rename --symbol <name> --map <i:from:to,…>`
//! - `rbit-msvc-4x rewrite <object> exchange --symbol <name> --at <i[:fields],…>`
//! - `rbit-msvc-4x propose <built> <reference> <name>`

mod coff;
mod correspond;
mod exchange;
mod layout;
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

const USAGE: &str = "usage: rbit-msvc-4x rewrite <object> <rewrite>… | rbit-msvc-4x propose <built> <reference> <name>, where each <rewrite> is schedule --symbol <name> --order <i,j,…>, rename --symbol <name> --map <i:from:to,…>, or exchange --symbol <name> --at <i[:fields],…>";

/// How many rewrites a proposal chains at most.
const CHAIN: usize = 3;

fn run(args: &[String]) -> Result<()> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["rewrite", object, rewrites @ ..] if !rewrites.is_empty() && rewrites.len() % 5 == 0 => {
            let mut bytes = fs::read(object).with_context(|| format!("cannot read {object}"))?;
            for rewrite in rewrites.chunks(5) {
                apply(&mut bytes, rewrite)?;
            }
            fs::write(object, bytes).with_context(|| format!("cannot write {object}"))
        }
        ["propose", built, reference, symbol] => {
            let (built, reference) = (fs::read(built)?, fs::read(reference)?);
            let wanted = Coff::parse(&reference)?.function(symbol)?.bytes;
            let proposals = if Coff::parse(&built)?.function(symbol)?.bytes == wanted {
                Vec::new()
            } else {
                search(&built, &reference, symbol, CHAIN)?
            };
            writeln!(std::io::stdout(), "{}", serde_json::to_string(&proposals)?)?;
            Ok(())
        }
        _ => bail!(USAGE),
    }
}

/// Applies one rewrite, `<name> --symbol <symbol> <flag> <value>`, to the
/// object in `bytes`.
fn apply(bytes: &mut Vec<u8>, rewrite: &[&str]) -> Result<()> {
    let [name, "--symbol", symbol, flag, value] = rewrite else {
        bail!(USAGE);
    };
    let function = Coff::parse(bytes)?.function(symbol)?;
    let code = match (*name, *flag) {
        ("schedule", "--order") => {
            let order = value
                .split(',')
                .map(|item| {
                    item.trim()
                        .parse::<usize>()
                        .with_context(|| format!("{item} is not an instruction index"))
                })
                .collect::<Result<Vec<_>>>()?;
            return reorder(bytes, symbol, &order);
        }
        ("rename", "--map") => {
            *bytes = rename::apply(bytes, &function, &rename::parse(value)?)?;
            return Ok(());
        }
        ("exchange", "--at") => exchange::apply(&function, &exchange::parse(value)?)?,
        _ => bail!(USAGE),
    };
    bytes[function.file..function.file + code.len()].copy_from_slice(&code);
    Ok(())
}

/// The chains of at most `depth` rewrites that turn `symbol` in `built`
/// into the one in `reference`. Each rewrite but the last may leave the
/// registers apart, for a later one to rename.
fn search(built: &[u8], reference: &[u8], symbol: &str, depth: usize) -> Result<Vec<Vec<String>>> {
    let exact = derive(built, reference, symbol, false)?;
    if !exact.is_empty() || depth <= 1 {
        return Ok(exact);
    }
    let mut chains = Vec::new();
    for first in derive(built, reference, symbol, true)? {
        let mut bytes = built.to_vec();
        let args: Vec<&str> = first.iter().map(String::as_str).collect();
        if apply(&mut bytes, &args).is_err() {
            continue;
        }
        for rest in search(&bytes, reference, symbol, depth - 1)? {
            chains.push([first.clone(), rest].concat());
        }
    }
    Ok(chains)
}

/// The single rewrites that turn `symbol` in `built` into the one in
/// `reference`, or, when `loose`, into one with the same instruction forms.
fn derive(built: &[u8], reference: &[u8], symbol: &str, loose: bool) -> Result<Vec<Vec<String>>> {
    let (ours, theirs) = (
        Coff::parse(built)?.function(symbol)?,
        Coff::parse(reference)?.function(symbol)?,
    );
    let order = propose(&ours, &theirs, loose)?.map(|order| {
        let order: Vec<String> = order.iter().map(ToString::to_string).collect();
        ("schedule", "--order", order.join(","))
    });
    let map = (!loose)
        .then(|| rename::derive(built, symbol, &ours, &theirs.bytes))
        .flatten()
        .map(|map| ("rename", "--map", rename::format(&map)));
    let sites = exchange::derive(&ours, &theirs.bytes, loose)
        .map(|sites| ("exchange", "--at", exchange::format(&sites)));
    Ok(order
        .into_iter()
        .chain(map)
        .chain(sites)
        .map(|(rewrite, flag, value)| {
            [rewrite, "--symbol", symbol, flag, &value]
                .map(str::to_owned)
                .to_vec()
        })
        .collect())
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

/// The order that turns `built` into `reference`, when both hold the same
/// instructions with the same relocations, up to the displacements that
/// rebasing `[esp+d]` operands changes. When `loose`, instructions match by
/// form, whatever registers they name.
fn propose(built: &Function, reference: &Function, loose: bool) -> Result<Option<Vec<usize>>> {
    if built.bytes.len() != reference.bytes.len() {
        return Ok(None);
    }
    let (ours, theirs) = (decode(built)?, decode(reference)?);
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
        let shape = if loose {
            correspond::decode_at(&instruction.bytes, 0)
                .map_or_else(|_| String::new(), |decoded| correspond::form(&decoded))
                .into_bytes()
        } else {
            instruction.shape()
        };
        (shape, instruction.slot(), relocations)
    };
    let mut used = vec![false; ours.len()];
    let mut order = Vec::with_capacity(ours.len());
    for wanted in &theirs {
        let wanted = key(reference, wanted);
        let Some(index) =
            (0..ours.len()).find(|&index| !used[index] && key(built, &ours[index]) == wanted)
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
