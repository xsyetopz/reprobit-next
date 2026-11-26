//! Commands. Each returns one JSON value (for `--json` and the MCP server)
//! and a short human summary, so both audiences read the same facts.

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use serde_json::Value;

use crate::catalog::Catalog;
use crate::project::{Loaded, Project};

mod project;
mod tool;
pub(crate) use tool::ToolCommand;

/// Rebuild old software byte for byte with its original compiler, and show
/// why the result can be trusted.
#[derive(Debug, Parser)]
#[command(
    name = "rbit",
    version,
    about,
    after_help = "Start with `rbit status` in a project, or `rbit tool list` to browse the catalog."
)]
pub(crate) struct Cli {
    /// Print machine-readable JSON on stdout.
    #[arg(long, global = true)]
    pub(crate) json: bool,
    /// Print only results and errors, not progress.
    #[arg(short, long, global = true)]
    pub(crate) quiet: bool,
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// Every command.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Start a project: write reprobit.toml and ignore the local state.
    Init {
        /// Project directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Catalog id of the compiler, such as msvc-4.2 or decompme/ps2/ee-gcc2.95.3-136.
        #[arg(long)]
        compiler: String,
        /// Project name; defaults to the directory name.
        #[arg(long)]
        name: Option<String>,
    },
    /// Install the project's compiler and tools, then pin them in reprobit.lock.
    Setup {
        /// Project directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Show what is ready and the next thing to do.
    Status {
        /// Project directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Incremental developer build; reuses unchanged steps.
    Build {
        /// Project directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Build from scratch, compare every target with its reference, and write the trust report.
    Verify {
        /// Project directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Show where the last build differs from the references, without rebuilding.
    Diff {
        /// Project directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Search declaration-only preludes until one function matches a reference object.
    Grind {
        /// Project directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// The compile step to adjust.
        #[arg(long)]
        step: String,
        /// The function's symbol, such as _transform.
        #[arg(long)]
        symbol: String,
        /// The reference object holding the original function.
        #[arg(long)]
        reference_object: PathBuf,
        /// Families to try: shape, pad, forward, extern, registers, or an add-on id. Default: all.
        #[arg(long = "family")]
        families: Vec<String>,
        /// Most candidates to compile.
        #[arg(long, default_value_t = 64)]
        limit: usize,
        /// Save the match into reprobit.toml.
        #[arg(long)]
        accept: bool,
    },
    /// After a source edit, drop each saved prelude the build no longer needs.
    Repair {
        /// Project directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Print compile steps from a `compile_commands.json`, as `CMake`, Ninja, Meson, and Bear
    /// write it, as TOML to append to reprobit.toml.
    Import {
        /// The compilation database.
        database: PathBuf,
        /// Project directory; sources must live under it.
        #[arg(long, default_value = ".")]
        dir: PathBuf,
    },
    /// Remove local build state; `--all` also drops the step cache and Wine prefix.
    Clean {
        /// Project directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Remove everything under the state directory.
        #[arg(long)]
        all: bool,
    },
    /// Browse, install, run, and remove catalog tools.
    #[command(subcommand)]
    Tool(ToolCommand),
    /// Print the JSON Schema for `project` (reprobit.toml) or `catalog`.
    Schema {
        /// project or catalog.
        name: String,
    },
    /// Serve these commands to AI agents over MCP on stdin and stdout.
    #[cfg(feature = "mcp")]
    Mcp,
}

/// A command's result.
#[derive(Debug)]
pub(crate) struct Outcome {
    pub(crate) ok: bool,
    pub(crate) json: Value,
    pub(crate) text: String,
}

fn done(json: Value, text: String) -> Outcome {
    Outcome {
        ok: true,
        json,
        text,
    }
}

/// Runs one command.
pub(crate) fn run(command: Command) -> Result<Outcome> {
    match command {
        Command::Init {
            dir,
            compiler,
            name,
        } => project::init(&dir, &compiler, name),
        Command::Setup { dir } => project::setup(&Loaded::open(&dir)?),
        Command::Status { dir } => project::status(&dir),
        Command::Build { dir } => project::build(&Loaded::open(&dir)?),
        Command::Verify { dir } => project::verify(&Loaded::open(&dir)?),
        Command::Diff { dir } => project::diff(&Loaded::open(&dir)?),
        Command::Grind {
            dir,
            step,
            symbol,
            reference_object,
            families,
            limit,
            accept,
        } => {
            let search = project::Search {
                step,
                symbol,
                reference_object,
                families,
                limit,
                accept,
            };
            project::grind(&dir, &search)
        }
        Command::Repair { dir } => project::repair(&Loaded::open(&dir)?),
        Command::Import { database, dir } => project::import(&database, &dir),
        Command::Clean { dir, all } => project::clean(&Loaded::open(&dir)?, all),
        Command::Tool(command) => tool::run(command),
        Command::Schema { name } => {
            let schema = match name.as_str() {
                "project" => schemars::schema_for!(Project),
                "catalog" => schemars::schema_for!(Catalog),
                other => bail!("no schema {other}; use project or catalog"),
            };
            let text = serde_json::to_string_pretty(&schema)?;
            Ok(done(serde_json::to_value(&schema)?, text))
        }
        #[cfg(feature = "mcp")]
        Command::Mcp => bail!("`rbit mcp` serves on stdin; it cannot run as a nested command"),
    }
}
