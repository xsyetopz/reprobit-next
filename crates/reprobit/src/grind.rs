//! `rbit grind`: a bounded search for the declaration-only prelude that makes
//! one function compile to the reference bytes. Candidates are tried from
//! the smallest rendered prelude up, so the first match is the cheapest one.
//! Each i386 candidate is also tried with every register rename, because a
//! prelude that fixes everything but register allocation is a match too.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::build;
use crate::catalog::Catalog;
use crate::project::{Loaded, Prelude};
use crate::rewrite::{self, Registers};
use crate::runner::{Mode, Stage};
use crate::ui;
use crate::{binary, verify};

/// What a grind found.
#[derive(Debug, Serialize)]
pub(crate) struct Outcome {
    pub(crate) step: String,
    pub(crate) symbol: String,
    /// Candidates compiled, the current prelude included.
    pub(crate) tried: usize,
    /// Whether the step already matches without a change.
    pub(crate) already_matches: bool,
    /// The cheapest match, if any.
    pub(crate) found: Option<Match>,
    /// Whether `found` was written to `reprobit.toml`.
    pub(crate) accepted: bool,
}

/// A prelude and register rename that make a function match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Match {
    /// The prelude, or `None` to compile without one.
    pub(crate) prelude: Option<Prelude>,
    /// Registers renamed after the compile; empty for none.
    pub(crate) registers: Registers,
}

/// Every candidate in the given families, smallest rendering first.
pub(crate) fn candidates(families: &[String]) -> Vec<Prelude> {
    let wants = |name: &str| families.is_empty() || families.iter().any(|family| family == name);
    let mut all = Vec::new();
    if wants("shape") {
        all.extend((1..=10).flat_map(|classes| {
            (classes..=classes * 10).map(move |functions| Prelude::Shape { classes, functions })
        }));
    }
    if wants("pad") {
        all.extend((1..=10).flat_map(|classes| {
            (1..=10).map(move |functions| Prelude::Pad { classes, functions })
        }));
    }
    if wants("forward") {
        all.extend((1..=100).map(|count| Prelude::Forward { count }));
    }
    if wants("extern") {
        all.extend((1..=100).map(|count| Prelude::Extern { count }));
    }
    all.sort_by_key(|prelude| prelude.render().len());
    all
}

/// Searches up to `limit` candidates for `step` until `symbol` matches the
/// same symbol in `reference_object`. With `accept`, the match is saved.
pub(crate) fn grind(
    loaded: &Loaded,
    step_id: &str,
    symbol: &str,
    reference_object: &Path,
    families: &[String],
    limit: usize,
    accept: bool,
) -> Result<Outcome> {
    let step = loaded.step(step_id)?.clone();
    ensure!(
        step.inputs.iter().all(|input| input.starts_with("source/")),
        "step {step_id} reads earlier outputs; grind a compile step"
    );
    let object = step.object()?.to_owned();
    let renames = families.is_empty() || families.iter().any(|family| family == "registers");
    let reference = fs::read(loaded.root.join(reference_object))
        .with_context(|| format!("cannot read {}", reference_object.display()))?;
    let wanted = binary::function(&reference, symbol).context("in the reference object")?;
    let catalog = Catalog::load()?;
    let compiler = build::compiler(loaded, &catalog)?;
    let mode = Mode::pick(
        loaded
            .project
            .toolchain
            .runs
            .as_ref()
            .unwrap_or(&compiler.tool.runs),
        &loaded.project.toolchain.runner,
    )?;
    let (headers, sources) = build::sources(loaded)?;
    let stage = Stage::create(
        &loaded.state().join("grind"),
        &loaded.root,
        &sources,
        &compiler.root,
        &loaded.project.paths(mode.dos()),
        mode,
        &loaded.state(),
    )?;
    let mut outcome = Outcome {
        step: step_id.into(),
        symbol: symbol.into(),
        tried: 0,
        already_matches: false,
        found: None,
        accepted: false,
    };
    let current = Match {
        prelude: step.prelude.clone(),
        registers: step.registers.get(symbol).cloned().unwrap_or_default(),
    };
    let current_prelude = current.prelude.clone();
    let queue = std::iter::once(current_prelude.clone()).chain(
        candidates(families)
            .into_iter()
            .filter(|candidate| Some(candidate) != current_prelude.as_ref())
            .map(Some),
    );
    for candidate in queue.take(limit.saturating_add(1)) {
        let mut trial = step.clone();
        trial.prelude.clone_from(&candidate);
        trial.registers.remove(symbol);
        outcome.tried += 1;
        ui::note(&format!(
            "[{}] {}",
            outcome.tried,
            candidate
                .as_ref()
                .map_or_else(|| "no prelude".to_owned(), describe)
        ));
        build::run_step(loaded, &catalog, &compiler, &stage, &trial, &headers, true)?;
        let built = fs::read(stage.build().join(&object))?;
        let Ok(function) = binary::function(&built, symbol) else {
            continue;
        };
        let registers = if function == wanted {
            Some(Registers::new())
        } else if renames && function.relocations == wanted.relocations {
            let allowed = rewrite::renamable(&built).unwrap_or_default();
            rewrite::permutations(allowed)
                .into_iter()
                .find(|registers| {
                    rewrite::rename(&function, registers, allowed)
                        .is_ok_and(|bytes| bytes == wanted.bytes)
                })
        } else {
            None
        };
        if let Some(registers) = registers {
            let found = Match {
                prelude: candidate,
                registers,
            };
            if found == current {
                outcome.already_matches = true;
            } else {
                outcome.found = Some(found);
            }
            break;
        }
    }
    stage.finish();
    if accept && let Some(found) = &outcome.found {
        loaded.set_prelude(step_id, found.prelude.as_ref())?;
        loaded.set_registers(step_id, symbol, &found.registers)?;
        outcome.accepted = true;
    }
    Ok(outcome)
}

/// What `rbit repair` changed.
#[derive(Debug, Serialize)]
pub(crate) struct Repair {
    /// Steps whose prelude was no longer needed and was removed.
    pub(crate) dropped: Vec<String>,
    /// Steps whose prelude is still needed.
    pub(crate) kept: Vec<String>,
    /// Whether every target matches after the repair.
    pub(crate) identical: bool,
}

/// After a source edit, drops each saved prelude the build no longer needs.
/// A prelude goes only when every target still matches without it, so a
/// repair never trades an exact build for a simpler project.
pub(crate) fn repair(loaded: &Loaded) -> Result<Repair> {
    let stage = loaded.state().join("repair");
    let matches = |project: &Loaded| -> Result<bool> {
        let built = build::run(project, &stage, true)?;
        let targets = verify::compare(project, &built.stage.build())?;
        Ok(targets.iter().all(|target| target.identical))
    };
    let mut current = loaded.clone();
    let mut dropped = Vec::new();
    let mut kept = Vec::new();
    let saved: Vec<String> = loaded
        .project
        .steps
        .iter()
        .filter(|step| step.prelude.is_some())
        .map(|step| step.id.clone())
        .collect();
    for id in saved {
        let mut trial = current.clone();
        trial
            .project
            .steps
            .iter_mut()
            .filter(|step| step.id == id)
            .for_each(|step| step.prelude = None);
        if matches(&trial)? {
            loaded.set_prelude(&id, None)?;
            dropped.push(id);
            current = trial;
        } else {
            kept.push(id);
        }
    }
    let identical = matches(&current)?;
    Ok(Repair {
        dropped,
        kept,
        identical,
    })
}

/// A one-line description of a match.
pub(crate) fn describe_match(found: &Match) -> String {
    let prelude = found
        .prelude
        .as_ref()
        .map_or_else(|| "no prelude".to_owned(), describe);
    if found.registers.is_empty() {
        return prelude;
    }
    let renames: Vec<String> = found
        .registers
        .iter()
        .map(|(from, to)| format!("{from}->{to}"))
        .collect();
    format!("{prelude}, registers {}", renames.join(" "))
}

/// A one-line description of a prelude.
pub(crate) fn describe(prelude: &Prelude) -> String {
    match prelude {
        Prelude::Shape { classes, functions } => {
            format!("shape classes={classes} functions={functions}")
        }
        Prelude::Pad { classes, functions } => {
            format!("pad classes={classes} functions={functions}")
        }
        Prelude::Forward { count } => format!("forward count={count}"),
        Prelude::Extern { count } => format!("extern count={count}"),
    }
}
