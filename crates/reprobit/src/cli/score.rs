//! `rbit score`: per-function grades of a built object against its reference,
//! or of every unit in a project that names reference objects.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::Outcome;
use super::objdiff::units;
use crate::project::Loaded;
use crate::score::{self, Grade, Summary};

fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("cannot read {}", path.display()))
}

/// What to grade: two objects, or a project's units.
pub(super) struct Request {
    pub(super) objects: Vec<PathBuf>,
    pub(super) dir: PathBuf,
    pub(super) symbols: Vec<String>,
}

pub(super) fn score(request: &Request) -> Result<Outcome> {
    let summary = match request.objects.as_slice() {
        [reference, built] => score::score(&read(reference)?, &read(built)?, &request.symbols)?,
        [] => project(&Loaded::open(&request.dir)?, &request.symbols)?,
        _ => bail!("give a reference object and a built object, or neither to score the project"),
    };
    let mut text = String::new();
    for item in &summary.symbols {
        let unit = item
            .unit
            .as_ref()
            .map_or_else(String::new, |unit| format!("{unit}: "));
        match item.grade {
            Grade::Exact => {}
            Grade::Reloc => writeln!(text, "reloc      {unit}{}", item.name)?,
            Grade::Missing => writeln!(text, "missing    {unit}{}", item.name)?,
            Grade::Different => writeln!(
                text,
                "different  {unit}{} ({:.2}% of bytes agree)",
                item.name, item.matching_percent
            )?,
        }
    }
    writeln!(
        text,
        "{:.2}% matched ({}/{}): {} exact, {} reloc, {} different, {} missing",
        summary.matched_percent,
        summary.exact + summary.reloc,
        summary.functions,
        summary.exact,
        summary.reloc,
        summary.different,
        summary.missing,
    )?;
    Ok(Outcome {
        ok: summary.different == 0 && summary.missing == 0,
        json: serde_json::to_value(&summary)?,
        text,
    })
}

/// Every unit's functions; a unit whose object is not built yet counts as missing.
fn project(loaded: &Loaded, symbols: &[String]) -> Result<Summary> {
    let units = units(loaded)?;
    if units.is_empty() {
        bail!(
            "no step names a reference-object; pass two objects, or add `reference-object` to each compile step"
        );
    }
    let mut scored = Vec::new();
    for unit in units {
        let reference = read(&loaded.root.join(&unit.reference))?;
        let built = loaded.root.join(&unit.built);
        let mut graded = if built.is_file() {
            let names: Vec<String> = if symbols.is_empty() {
                Vec::new()
            } else {
                let defined = crate::binary::functions(&reference)?;
                symbols
                    .iter()
                    .filter(|name| defined.contains(name))
                    .cloned()
                    .collect()
            };
            if !symbols.is_empty() && names.is_empty() {
                continue;
            }
            score::score(&reference, &read(&built)?, &names)?.symbols
        } else {
            score::unbuilt(&reference)?
                .into_iter()
                .filter(|item| symbols.is_empty() || symbols.contains(&item.name))
                .collect()
        };
        for item in &mut graded {
            item.unit = Some(unit.step.clone());
        }
        scored.extend(graded);
    }
    if scored.is_empty() {
        bail!("no unit defines {}", symbols.join(", "));
    }
    Ok(score::total(scored))
}
