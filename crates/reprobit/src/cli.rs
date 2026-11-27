//! Commands. Each returns one JSON value (for `--json` and the MCP server)
//! and a short human summary, so both audiences read the same facts.

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use serde_json::Value;

use crate::catalog::Catalog;
use crate::project::{Loaded, Project};

mod behave;
mod objdiff;
mod project;
mod score;
mod space;
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
        /// An object the build must write, as objdiff passes it; the build fails when
        /// no step writes it. Every changed step still rebuilds.
        #[arg(long)]
        object: Option<PathBuf>,
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
    /// Grade each function of a built object against its reference object: exact, reloc
    /// (only relocation targets differ), different, or missing. With no objects, grade
    /// every step that names a reference-object.
    Score {
        /// The object holding the original functions, then the object the build wrote.
        #[arg(num_args = 0..=2, value_names = ["REFERENCE", "BUILT"])]
        objects: Vec<PathBuf>,
        /// Project directory, when no objects are given.
        #[arg(long, default_value = ".")]
        dir: PathBuf,
        /// Functions to grade; default every function the reference defines.
        #[arg(long = "symbol")]
        symbols: Vec<String>,
        /// Fail only when a function recorded in reprobit.matched stopped matching.
        #[arg(long)]
        regressions: bool,
        /// Drop graded functions that no longer match from reprobit.matched.
        #[arg(long)]
        forget: bool,
    },
    /// Run each `[[check]]`: one command against a target's reference and its artifact, whose
    /// exit code and output must agree. For recompiled targets, whose bytes differ by design.
    Behave {
        /// Project directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Checks to run; default all.
        #[arg(long = "check")]
        checks: Vec<String>,
    },
    /// Write objdiff.json for objdiff and `objdiff-cli report`, from each step's
    /// reference-object.
    Objdiff {
        /// Project directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
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
    /// Show what reprobit keeps on disk and how much `rbit gc` would free.
    Space {
        /// Project directory; its state is included when it holds a reprobit.toml.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Step-cache entries unused for more days than this count as reclaimable.
        #[arg(long, default_value_t = 14)]
        keep_days: u64,
    },
    /// Free disk space: downloads, tool versions the catalog dropped, project scratch, and idle
    /// step-cache entries.
    Gc {
        /// Project directory; its state is included when it holds a reprobit.toml.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Keep step-cache entries used within this many days.
        #[arg(long, default_value_t = 14)]
        keep_days: u64,
        /// Also remove superseded runner images from docker or podman.
        #[arg(long)]
        images: bool,
        /// List what would go and remove nothing.
        #[arg(long)]
        dry_run: bool,
    },
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
        Command::Build { dir, object } => project::build(&Loaded::open(&dir)?, object.as_deref()),
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
        Command::Score {
            objects,
            dir,
            symbols,
            regressions,
            forget,
        } => score::score(&score::Request {
            objects,
            dir,
            symbols,
            regressions,
            forget,
        }),
        Command::Behave { dir, checks } => behave::behave(&Loaded::open(&dir)?, &checks),
        Command::Objdiff { dir } => objdiff::objdiff(&Loaded::open(&dir)?),
        Command::Repair { dir } => project::repair(&Loaded::open(&dir)?),
        Command::Import { database, dir } => project::import(&database, &dir),
        Command::Clean { dir, all } => project::clean(&Loaded::open(&dir)?, all),
        Command::Tool(command) => tool::run(command),
        Command::Space { dir, keep_days } => space::space(&dir, keep_days),
        Command::Gc {
            dir,
            keep_days,
            images,
            dry_run,
        } => space::gc(&dir, keep_days, images, dry_run),
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
