//! `rbit objdiff`: writes the project's `objdiff.json`, so objdiff's GUI and
//! `objdiff-cli report` see each compile step that names a reference object.
//! objdiff rebuilds through `rbit build --object`, which keeps every build
//! inside the declared-input stage.

use std::fs;
use std::path::Path;

use anyhow::{Result, ensure};
use serde_json::{Value, json};

use super::project::runner_build;
use super::{Outcome, done};
use crate::project::Loaded;

/// objdiff's configuration file, which it looks for at the project root.
pub(super) const CONFIG: &str = "objdiff.json";

const SCHEMA: &str = "https://raw.githubusercontent.com/encounter/objdiff/main/config.schema.json";

/// A compile step paired with its original object, paths relative to the project.
pub(super) struct Unit {
    pub(super) step: String,
    pub(super) reference: String,
    pub(super) built: String,
    pub(super) source: Option<String>,
}

/// Every step with a `reference-object`, in step order.
pub(super) fn units(loaded: &Loaded) -> Result<Vec<Unit>> {
    let build = Path::new(&loaded.project.state)
        .join("stage")
        .join(runner_build(loaded));
    loaded
        .project
        .steps
        .iter()
        .filter_map(|step| Some((step, step.reference_object.clone()?)))
        .map(|(step, reference)| {
            let built = build.join(step.object()?);
            Ok(Unit {
                step: step.id.clone(),
                reference,
                built: built.to_string_lossy().replace('\\', "/"),
                source: step
                    .inputs
                    .iter()
                    .find_map(|input| input.strip_prefix("source/"))
                    .map(str::to_owned),
            })
        })
        .collect()
}

/// The `objdiff.json` value for `loaded`.
pub(super) fn config(loaded: &Loaded) -> Result<Value> {
    let units = units(loaded)?;
    ensure!(
        !units.is_empty(),
        "no step names a reference-object; add `reference-object = \"reference/obj/<file>.obj\"` to each compile step"
    );
    let units: Vec<Value> = units
        .iter()
        .map(|unit| {
            let mut entry = json!({
                "name": unit.step,
                "target_path": unit.reference,
                "base_path": unit.built,
            });
            if let Some(source) = &unit.source {
                entry["metadata"] = json!({ "source_path": source });
            }
            entry
        })
        .collect();
    Ok(json!({
        "$schema": SCHEMA,
        "custom_make": "rbit",
        "custom_args": ["build", "--quiet", "--object"],
        "build_target": false,
        "build_base": true,
        "ignore_patterns": [format!("{}/**/*", loaded.project.state.trim_end_matches('/'))],
        "units": units,
    }))
}

pub(super) fn objdiff(loaded: &Loaded) -> Result<Outcome> {
    let value = config(loaded)?;
    let path = loaded.root.join(CONFIG);
    fs::write(&path, serde_json::to_string_pretty(&value)? + "\n")?;
    let count = value["units"].as_array().map_or(0, Vec::len);
    Ok(done(
        json!({ "path": path, "units": count }),
        format!("Wrote {} with {count} units", path.display()),
    ))
}
