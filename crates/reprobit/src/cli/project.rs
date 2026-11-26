//! The project commands: init, setup, status, build, verify, diff, grind, repair, import, and clean.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::{Outcome, done};
use crate::catalog::Catalog;
use crate::project::{FILE, LOCK, Loaded, Step};
use crate::runner::Mode;
use crate::{binary, build, grind as search, tools, verify as check};

/// What `rbit grind` searches for.
pub(super) struct Search {
    pub(super) step: String,
    pub(super) symbol: String,
    pub(super) reference_object: PathBuf,
    pub(super) families: Vec<String>,
    pub(super) limit: usize,
    pub(super) accept: bool,
}

pub(super) fn build(loaded: &Loaded) -> Result<Outcome> {
    let built = build::run(loaded, &loaded.state().join("stage"), true)?;
    let cached = built
        .receipts
        .iter()
        .filter(|receipt| receipt.cached)
        .count();
    let artifacts: Vec<String> = loaded
        .project
        .targets
        .iter()
        .map(|target| {
            built
                .stage
                .build()
                .join(&target.artifact)
                .display()
                .to_string()
        })
        .collect();
    let text = format!(
        "Built {} steps ({cached} from cache)\n{}",
        built.receipts.len(),
        artifacts.join("\n")
    );
    Ok(done(
        json!({ "steps": built.receipts, "artifacts": artifacts }),
        text,
    ))
}

pub(super) fn verify(loaded: &Loaded) -> Result<Outcome> {
    let report = check::verify(loaded)?;
    let same = report
        .targets
        .iter()
        .filter(|target| target.identical)
        .count();
    let verdict = if report.identical { "passed" } else { "failed" };
    let mut text = format!(
        "Verification {verdict}: {same}/{} targets are byte-identical\n",
        report.targets.len()
    );
    text.push_str(&differences_text(&report.targets)?);
    write!(
        text,
        "Report: {}",
        check::report_dir(loaded).join("report.html").display()
    )?;
    Ok(Outcome {
        ok: report.identical,
        json: serde_json::to_value(&report)?,
        text,
    })
}

pub(super) fn diff(loaded: &Loaded) -> Result<Outcome> {
    // The newer of the developer build and the last `rbit verify` build,
    // which is where `rbit status` sends an agent after a failed verification.
    let relative = runner_build(loaded);
    let artifact = loaded
        .project
        .targets
        .first()
        .map_or_default(|target| target.artifact.clone());
    let build = ["stage", "verify"]
        .into_iter()
        .map(|stage| loaded.state().join(stage).join(&relative))
        .filter_map(|build| {
            let built = fs::metadata(build.join(&artifact)).ok()?.modified().ok()?;
            Some((built, build))
        })
        .max()
        .map(|(_, build)| build)
        .context("nothing is built yet; run `rbit build`")?;
    let targets = check::compare(loaded, &build)?;
    let ok = targets.iter().all(|target| target.identical);
    let text = if ok {
        "Every target matches its reference".to_owned()
    } else {
        differences_text(&targets)?
    };
    Ok(Outcome {
        ok,
        json: serde_json::to_value(&targets)?,
        text,
    })
}

pub(super) fn grind(dir: &Path, wanted: &Search) -> Result<Outcome> {
    let loaded = Loaded::open(dir)?;
    let Search {
        step,
        symbol,
        reference_object,
        families,
        limit,
        accept,
    } = wanted;
    let outcome = search::grind(
        &loaded,
        step,
        symbol,
        reference_object,
        families,
        *limit,
        *accept,
    )?;
    let text = match (&outcome.found, outcome.already_matches) {
        (_, true) => format!("{symbol} already matches; nothing to change"),
        (Some(found), _) if outcome.accepted => {
            format!(
                "Saved {} to step {step}; run `rbit verify`",
                search::describe_match(found)
            )
        }
        (Some(found), _) => format!(
            "Found {} after {} candidates. Save it with:\n  rbit grind {} --step {step} --symbol {symbol} --reference-object {} --accept",
            search::describe_match(found),
            outcome.tried,
            dir.display(),
            reference_object.display()
        ),
        (None, false) => format!(
            "Nothing matched {symbol} in {} candidates; raise --limit or try other families",
            outcome.tried
        ),
    };
    Ok(Outcome {
        ok: outcome.found.is_some() || outcome.already_matches,
        json: serde_json::to_value(&outcome)?,
        text,
    })
}

pub(super) fn repair(loaded: &Loaded) -> Result<Outcome> {
    let repair = search::repair(loaded)?;
    let mut text = match repair.dropped.as_slice() {
        [] => "Every saved prelude is still needed".to_owned(),
        dropped => format!("Dropped preludes no longer needed: {}", dropped.join(", ")),
    };
    text.push_str(if repair.identical {
        "\nEvery target matches; run `rbit verify` for the report"
    } else {
        "\nSome targets differ; run `rbit diff`, then `rbit grind` on the function that moved"
    });
    Ok(Outcome {
        ok: repair.identical,
        json: serde_json::to_value(&repair)?,
        text,
    })
}

pub(super) fn import(database: &Path, dir: &Path) -> Result<Outcome> {
    #[derive(serde::Serialize)]
    struct Steps<'a> {
        step: &'a [Step],
    }
    let steps = crate::import::compile_commands(database, dir)?;
    let text = toml::to_string(&Steps { step: &steps })?;
    Ok(done(serde_json::to_value(&steps)?, text))
}

pub(super) fn clean(loaded: &Loaded, all: bool) -> Result<Outcome> {
    let state = loaded.state();
    let names: &[&str] = if all {
        &[""]
    } else {
        &["stage", "verify", "grind", "reports", "logs"]
    };
    for name in names {
        let path = state.join(name);
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
    }
    Ok(done(
        json!({ "removed": state }),
        format!("Cleaned {}", state.display()),
    ))
}

fn runner_build(loaded: &Loaded) -> String {
    // Both default spellings put the build directory at `build`.
    let build = loaded
        .project
        .paths
        .as_ref()
        .map_or("build", |paths| paths.build.as_str());
    build
        .split_once(':')
        .map_or(build, |(_, rest)| rest)
        .trim_start_matches(['\\', '/'])
        .replace('\\', "/")
}

fn differences_text(targets: &[check::Comparison]) -> Result<String> {
    let mut text = String::new();
    for target in targets.iter().filter(|target| !target.identical) {
        writeln!(
            text,
            "  {}: {} bytes vs reference {} bytes",
            target.id, target.size, target.reference_size
        )?;
        for difference in &target.differences {
            writeln!(
                text,
                "    offset {:#x}, {} bytes{}{}",
                difference.offset,
                difference.length,
                difference
                    .section
                    .as_ref()
                    .map_or_default(|name| format!(" in {name}")),
                difference
                    .rva
                    .map_or_default(|rva| format!(" (rva {rva:#x})")),
            )?;
        }
    }
    Ok(text)
}

pub(super) fn init(dir: &Path, compiler: &str, name: Option<String>) -> Result<Outcome> {
    let path = dir.join(FILE);
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    Catalog::load()?.get(compiler)?;
    fs::create_dir_all(dir)?;
    let name = name.unwrap_or_else(|| {
        dir.canonicalize()
            .ok()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "project".into())
    });
    fs::write(
        &path,
        format!(
            "#:schema https://raw.githubusercontent.com/xsyetopz/reprobit-next/master/schemas/project.schema.json\n\
name = \"{name}\"\n\n[toolchain]\ncompiler = \"{compiler}\"\n\n\
# Add one [[step]] per compiler, librarian, or linker run, then one [[target]]\n\
# per binary to reproduce. See docs/project.md.\n"
        ),
    )?;
    let ignore = dir.join(".gitignore");
    let existing = fs::read_to_string(&ignore).unwrap_or_default();
    if !existing.lines().any(|line| line.trim() == "/.reprobit/") {
        fs::write(
            &ignore,
            format!(
                "{existing}{}/.reprobit/\n",
                if existing.is_empty() || existing.ends_with('\n') {
                    ""
                } else {
                    "\n"
                }
            ),
        )?;
    }
    Ok(done(
        json!({ "created": path }),
        format!(
            "Created {}\nNext: add steps and targets, then run `rbit setup`",
            path.display()
        ),
    ))
}

pub(super) fn setup(loaded: &Loaded) -> Result<Outcome> {
    let catalog = Catalog::load()?;
    let toolchain = &loaded.project.toolchain;
    let mut lock =
        String::from("# Written by `rbit setup`. Pins the exact installed content of each tool.\n");
    let pinned = build::locked(&loaded.root)?;
    let mut installed = Vec::new();
    for id in std::iter::once(&toolchain.compiler).chain(&toolchain.tools) {
        let tool = tools::install(&catalog, id)?;
        if let Some(expected) = pinned.get(id)
            && *expected != tool.receipt.tree
        {
            bail!(
                "{id} installed with content {}, but {LOCK} pins {expected}; the upstream download changed",
                tool.receipt.tree
            );
        }
        write!(
            lock,
            "\n[tool.\"{id}\"]\nversion = \"{}\"\ntree = \"{}\"\n",
            tool.receipt.version, tool.receipt.tree
        )?;
        if !tool.receipt.pins.is_empty() {
            writeln!(lock, "pins = {:?}", tool.receipt.pins)?;
        }
        installed.push(json!({ "id": id, "path": tool.root, "tree": tool.receipt.tree }));
    }
    fs::write(loaded.root.join(LOCK), lock)?;
    let report = status(&loaded.root)?;
    Ok(Outcome {
        ok: report.ok,
        json: json!({ "installed": installed, "status": report.json }),
        text: report.text,
    })
}

pub(super) fn status(dir: &Path) -> Result<Outcome> {
    let loaded = Loaded::open(dir)?;
    let catalog = Catalog::load()?;
    let mut checks: Vec<(String, bool, String)> = Vec::new();
    let compiler = build::compiler(&loaded, &catalog);
    checks.push(match &compiler {
        Ok(found) => (
            format!("compiler {}", loaded.project.toolchain.compiler),
            true,
            found.root.display().to_string(),
        ),
        Err(error) => (
            format!("compiler {}", loaded.project.toolchain.compiler),
            false,
            format!("{error:#}"),
        ),
    });
    if let Ok(found) = &compiler {
        checks.push(
            match Mode::pick(
                loaded
                    .project
                    .toolchain
                    .runs
                    .as_ref()
                    .unwrap_or(&found.tool.runs),
                &loaded.project.toolchain.runner,
            ) {
                Ok(mode) => ("runner".into(), true, mode.name().into()),
                Err(error) => ("runner".into(), false, format!("{error:#}")),
            },
        );
    }
    for id in &loaded.project.toolchain.tools {
        let found = tools::find(&catalog, id)?;
        checks.push((
            format!("tool {id}"),
            found.is_some(),
            if found.is_some() {
                "installed".into()
            } else {
                "run `rbit setup`".into()
            },
        ));
    }
    checks.push((
        "steps".into(),
        !loaded.project.steps.is_empty(),
        format!("{} declared", loaded.project.steps.len()),
    ));
    checks.extend(reference_checks(&loaded));
    let last = check::report_dir(&loaded).join("report.json");
    let verified = fs::read_to_string(&last)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|report| report["identical"].as_bool());
    let ready = checks.iter().all(|(_, ok, _)| *ok);
    let next = checks.iter().find(|(_, ok, _)| !ok).map_or_else(
        || match verified {
            Some(true) => "nothing; the last verification passed".to_owned(),
            Some(false) => {
                "`rbit diff` to see what differs, or `rbit grind` on a mismatched function"
                    .to_owned()
            }
            None => "`rbit verify`".to_owned(),
        },
        |(name, _, detail)| format!("fix {name}: {detail}"),
    );
    let mut text = String::new();
    for (name, ok, detail) in &checks {
        writeln!(
            text,
            "{} {name}: {detail}",
            if *ok { "ok  " } else { "todo" }
        )?;
    }
    write!(text, "Next: {next}")?;
    let json = json!({
        "ready": ready,
        "verified": verified,
        "next": next,
        "checks": checks.iter().map(|(name, ok, detail)| json!({ "name": name, "ok": ok, "detail": detail })).collect::<Vec<_>>(),
    });
    Ok(Outcome {
        ok: ready,
        json,
        text,
    })
}

/// One status line per target: whether its original is present and pinned.
fn reference_checks(loaded: &Loaded) -> Vec<(String, bool, String)> {
    let mut checks = Vec::new();
    for target in &loaded.project.targets {
        let path = loaded.root.join(&target.reference);
        let detail = match fs::read(&path) {
            Err(_) => Err(format!("place the original at {}", target.reference)),
            Ok(mut bytes) => binary::normalize(&mut bytes, &target.normalize)
                .and_then(|_| check::check_reference(target, &bytes))
                .map(|()| target.reference.clone())
                .map_err(|error| format!("{error:#}")),
        };
        checks.push((
            format!("reference {}", target.id),
            detail.is_ok(),
            detail.unwrap_or_else(|error| error),
        ));
    }
    checks
}
