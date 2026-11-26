//! `rbit tool`: browsing the catalog and managing installed tools.

use std::fmt::Write as _;
use std::fs;

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde_json::{Value, json};

use super::{Outcome, done};
use crate::catalog::Catalog;
use crate::runner::Mode;
use crate::{home, tools};

/// Tool subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum ToolCommand {
    /// List catalog tools; filter by text, tag, or kind.
    List {
        /// Text to look for in ids, summaries, and tags.
        filter: Option<String>,
        /// Only installed tools.
        #[arg(long)]
        installed: bool,
    },
    /// Show one tool's catalog entry and install state.
    Info {
        /// Tool id.
        id: String,
    },
    /// Download, check, and unpack tools.
    Install {
        /// Tool ids.
        #[arg(required = true)]
        ids: Vec<String>,
    },
    /// Delete installed tools; `--all` deletes every tool and download.
    Remove {
        /// Tool ids.
        ids: Vec<String>,
        /// Remove everything reprobit downloaded.
        #[arg(long, conflicts_with = "ids")]
        all: bool,
    },
    /// Print the path of a tool's command.
    Which {
        /// Tool id.
        id: String,
        /// Command name, when the tool has several.
        command: Option<String>,
    },
    /// Run a tool's command with arguments, installing it first if needed.
    Run {
        /// Tool id, or id:command.
        id: String,
        /// Arguments for the command.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Print the directory that holds every installed tool.
    Home,
}

/// Runs one `rbit tool` subcommand.
pub(super) fn run(command: ToolCommand) -> Result<Outcome> {
    let catalog = Catalog::load()?;
    match command {
        ToolCommand::List { filter, installed } => list(&catalog, filter.as_deref(), installed),
        ToolCommand::Info { id } => {
            let entry = catalog.get(&id)?;
            let found = tools::find(&catalog, &id)?;
            let path = found.as_ref().map(|found| found.root.display().to_string());
            let text = format!(
                "{id} {} ({})\n{}\n{}\nlicense: {}\nruns: {}\ncommands: {}\ninstalled: {}",
                entry.version,
                entry.kind,
                entry.summary,
                entry.homepage,
                entry.license,
                entry.runs,
                entry.bin.keys().cloned().collect::<Vec<_>>().join(", "),
                path.as_deref().unwrap_or("no"),
            );
            Ok(done(
                json!({ "id": id, "entry": entry, "installed": path }),
                text,
            ))
        }
        ToolCommand::Install { ids } => {
            let mut rows = Vec::new();
            for id in &ids {
                let installed = tools::install(&catalog, id)?;
                rows.push(
                    json!({ "id": id, "path": installed.root, "tree": installed.receipt.tree }),
                );
            }
            let text = rows
                .iter()
                .map(|row| {
                    format!(
                        "{} -> {}",
                        row["id"].as_str().unwrap_or_default(),
                        row["path"].as_str().unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            Ok(done(Value::Array(rows), text))
        }
        ToolCommand::Remove { ids, all } => remove(&catalog, &ids, all),
        ToolCommand::Which { id, command } => {
            let installed = tools::find(&catalog, &id)?
                .with_context(|| format!("{id} is not installed; run `rbit tool install {id}`"))?;
            let path = tools::command(&installed, command.as_deref())?;
            Ok(done(json!({ "path": path }), path.display().to_string()))
        }
        ToolCommand::Run { id, args } => run_tool(&catalog, &id, &args),
        ToolCommand::Home => {
            let root = home::root();
            Ok(done(json!({ "path": root }), root.display().to_string()))
        }
    }
}

fn list(catalog: &Catalog, filter: Option<&str>, installed: bool) -> Result<Outcome> {
    let mut rows = Vec::new();
    let mut text = String::new();
    for id in catalog.ids() {
        let entry = catalog.get(&id)?;
        let haystack = format!(
            "{id} {} {} {}",
            entry.summary,
            entry.tags.join(" "),
            entry.kind
        )
        .to_lowercase();
        if filter
            .as_ref()
            .is_some_and(|filter| !haystack.contains(&filter.to_lowercase()))
        {
            continue;
        }
        let present = tools::find(catalog, &id)?.is_some();
        if installed && !present {
            continue;
        }
        writeln!(
            text,
            "{}{id:<40} {:<9} {}",
            if present { "* " } else { "  " },
            entry.kind,
            entry.summary
        )?;
        rows.push(json!({ "id": id, "kind": entry.kind, "version": entry.version, "summary": entry.summary, "tags": entry.tags, "installed": present }));
    }
    Ok(done(Value::Array(rows), text))
}

fn remove(catalog: &Catalog, ids: &[String], all: bool) -> Result<Outcome> {
    if all {
        let root = home::root();
        for name in ["tools", "downloads"] {
            if root.join(name).exists() {
                fs::remove_dir_all(root.join(name))?;
            }
        }
        return Ok(done(
            json!({ "removed": root }),
            format!("Removed every tool and download under {}", root.display()),
        ));
    }
    if ids.is_empty() {
        bail!("name tools to remove, or pass --all");
    }
    let mut removed = Vec::new();
    for id in ids {
        if tools::remove(catalog, id)? {
            removed.push(id.clone());
        }
    }
    Ok(done(
        json!({ "removed": removed }),
        if removed.is_empty() {
            "Nothing was installed".into()
        } else {
            format!("Removed {}", removed.join(", "))
        },
    ))
}

fn run_tool(catalog: &Catalog, id: &str, args: &[String]) -> Result<Outcome> {
    let (id, name) = id
        .split_once(':')
        .map_or((id, None), |(id, name)| (id, Some(name)));
    let installed = tools::install(catalog, id)?;
    let path = tools::command(&installed, name)?;
    let mut command = match Mode::pick(&installed.tool.runs, "auto")? {
        Mode::Wine(wine) => {
            let mut command = std::process::Command::new(wine);
            command.arg(&path);
            command
        }
        Mode::Wibo(wibo) => {
            let mut command = std::process::Command::new(wibo);
            command.arg(&path);
            command
        }
        Mode::Docker { .. } => bail!("{id} runs in a container; call it from a project step"),
        _ => tools::launch(&path),
    };
    let status = command
        .args(args)
        .status()
        .with_context(|| format!("cannot start {}", path.display()))?;
    #[cfg(unix)]
    let signal = std::os::unix::process::ExitStatusExt::signal(&status);
    #[cfg(not(unix))]
    let signal: Option<i32> = None;
    // A tool killed by a signal has no exit code; say which signal, since
    // macOS kills a binary whose code signature no longer holds.
    let text = signal.map_or_default(|signal| format!("{id} was killed by signal {signal}"));
    Ok(Outcome {
        ok: status.success(),
        json: json!({ "status": status.code(), "signal": signal }),
        text,
    })
}
