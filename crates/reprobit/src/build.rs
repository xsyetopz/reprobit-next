//! Running a project's steps. A developer build reuses cached steps whose
//! inputs are unchanged; a cold build (verification) never does. Either way
//! each step leaves a receipt with the digests of what it read and wrote.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use serde_json::json;

use crate::addon::Addon;
use crate::catalog::Catalog;
use crate::project::{LOCK, Loaded, Prelude, Step};
use crate::rewrite;
use crate::runner::{Mode, Stage};
use crate::tools::{self, Installed};
use crate::{binary, digest, grind, ui};

/// What one step read and wrote.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct StepReceipt {
    /// Step id.
    pub(crate) step: String,
    /// Runner that executed it.
    pub(crate) runner: String,
    /// Whether the outputs came from the developer cache.
    pub(crate) cached: bool,
    /// SHA-256 of each input, keyed by `source/…` or `build/…` path.
    pub(crate) inputs: BTreeMap<String, String>,
    /// SHA-256 of each output, keyed by build-relative path.
    pub(crate) outputs: BTreeMap<String, String>,
    /// What besides the compiler shaped the outputs: the prelude, register
    /// renames, and add-on rewrites, each with the add-on's content digest.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) adjustments: Vec<String>,
}

/// A finished build.
#[derive(Debug)]
pub(crate) struct Built {
    pub(crate) stage: Stage,
    pub(crate) compiler: Installed,
    pub(crate) receipts: Vec<StepReceipt>,
}

/// The project's compiler, installed and matching `reprobit.lock` when one exists.
pub(crate) fn compiler(loaded: &Loaded, catalog: &Catalog) -> Result<Installed> {
    let id = &loaded.project.toolchain.compiler;
    let installed = tools::find(catalog, id)?
        .with_context(|| format!("{id} is not installed; run `rbit setup`"))?;
    // The receipt records what was installed; hashing again catches later edits.
    let actual = digest::tree(&installed.root, &[tools::RECEIPT])?;
    let pinned = locked(&loaded.root)?
        .get(id)
        .cloned()
        .unwrap_or_else(|| installed.receipt.tree.clone());
    if actual != pinned {
        bail!(
            "{id} at {} has content {actual}, but {LOCK} pins {pinned}; reinstall it with `rbit tool remove {id}` and `rbit setup`",
            installed.root.display()
        );
    }
    Ok(installed)
}

/// The tree digest pinned for each tool id in `reprobit.lock`.
pub(crate) fn locked(root: &Path) -> Result<BTreeMap<String, String>> {
    let path = root.join(LOCK);
    if !path.is_file() {
        return Ok(BTreeMap::new());
    }
    let document: toml::Table = toml::from_str(&fs::read_to_string(&path)?)?;
    Ok(document
        .get("tool")
        .and_then(toml::Value::as_table)
        .map_or_default(|tools| {
            tools
                .iter()
                .filter_map(|(id, entry)| {
                    Some((id.clone(), entry.get("tree")?.as_str()?.to_owned()))
                })
                .collect()
        }))
}

/// A stage at `root` holding the files the producer may see, and the
/// digest of the project's headers.
pub(crate) fn stage(loaded: &Loaded, compiler: &Installed, root: &Path) -> Result<(Stage, String)> {
    let project = &loaded.project;
    let mode = Mode::pick(
        project
            .toolchain
            .runs
            .as_ref()
            .unwrap_or(&compiler.tool.runs),
        &project.toolchain.runner,
    )?;
    let (headers, sources) = sources(loaded)?;
    let stage = Stage::create(
        root,
        &loaded.root,
        &sources,
        &compiler.root,
        &project.paths(mode.dos()),
        mode,
        &loaded.state(),
    )?;
    Ok((stage, headers))
}

/// Runs every step into `stage_root`. With `cache`, unchanged steps are
/// restored from the developer cache instead of being run.
pub(crate) fn run(loaded: &Loaded, stage_root: &Path, cache: bool) -> Result<Built> {
    let catalog = Catalog::load()?;
    let compiler = compiler(loaded, &catalog)?;
    let project = &loaded.project;
    let (stage, headers) = stage(loaded, &compiler, stage_root)?;
    let mut receipts = Vec::new();
    let total = project.steps.len();
    for (index, step) in project.steps.iter().enumerate() {
        ui::note(&format!("[{}/{total}] {}", index + 1, step.id));
        receipts.push(run_step(
            loaded, &catalog, &compiler, &stage, step, &headers, cache,
        )?);
    }
    for target in project
        .targets
        .iter()
        .filter(|target| !target.stamp.is_empty())
    {
        let path = stage.build().join(&target.artifact);
        let mut bytes =
            fs::read(&path).with_context(|| format!("target {} was not built", target.id))?;
        binary::stamp(&mut bytes, &target.stamp)
            .with_context(|| format!("cannot stamp target {}", target.id))?;
        fs::write(&path, bytes)?;
    }
    stage.finish();
    Ok(Built {
        stage,
        compiler,
        receipts,
    })
}

/// Runs one step, optionally with a replacement prelude, into `stage`.
pub(crate) fn run_step(
    loaded: &Loaded,
    catalog: &Catalog,
    compiler: &Installed,
    stage: &Stage,
    step: &Step,
    headers: &str,
    cache: bool,
) -> Result<StepReceipt> {
    let build = stage.build();
    let mut inputs = BTreeMap::new();
    for input in &step.inputs {
        let path = match input.strip_prefix("build/") {
            Some(built) => build.join(built),
            None => loaded.root.join(input.trim_start_matches("source/")),
        };
        inputs.insert(
            input.clone(),
            digest::file(&path).with_context(|| format!("step {} input {input}", step.id))?,
        );
    }
    let (program, inside, tool_tree) = resolve(catalog, compiler, &step.program)?;
    let args = with_prelude(compiler, stage, step)?;
    let addons = step
        .rewrite
        .iter()
        .map(|rewrite| Addon::open(catalog, loaded, &rewrite.addon))
        .collect::<Result<Vec<_>>>()?;
    let mut env = compiler.tool.env.clone();
    env.extend(step.env.clone());
    // `args` carry only the prelude's path, so its text joins the key.
    let key = digest::bytes(
        json!({
            "program": step.program, "tool": tool_tree, "args": args, "env": env,
            "prelude": step.prelude.as_ref().map(Prelude::render), "registers": step.registers, "rbit": env!("CARGO_PKG_VERSION"),
            "rewrite": step.rewrite, "addons": addons.iter().map(|addon| &addon.tree).collect::<Vec<_>>(),
            "inputs": inputs, "headers": headers, "runner": stage.mode().name(), "outputs": step.outputs,
        })
        .to_string()
        .as_bytes(),
    );
    let cached_dir = loaded.state().join("cache").join(&key);
    let cached = cache
        && step
            .outputs
            .iter()
            .all(|output| cached_dir.join(output).is_file());
    if cached {
        for output in &step.outputs {
            copy(&cached_dir.join(output), &build.join(output))?;
        }
    } else {
        for output in &step.outputs {
            // A stale output from an earlier run must not pass for this run's.
            if let Err(error) = fs::remove_file(build.join(output))
                && error.kind() != std::io::ErrorKind::NotFound
            {
                return Err(error.into());
            }
            if let Some(parent) = build.join(output).parent() {
                fs::create_dir_all(parent)?;
            }
        }
        stage.run(&step.id, &program, inside, &args, &env)?;
        if !step.registers.is_empty() {
            let path = build.join(step.object()?);
            let renamed = rewrite::rename_object(fs::read(&path)?, &step.registers)
                .with_context(|| format!("step {}", step.id))?;
            fs::write(&path, renamed)?;
        }
        for (addon, rewrite) in addons.iter().zip(&step.rewrite) {
            addon
                .rewrite(&build.join(step.object()?), &rewrite.args)
                .with_context(|| format!("step {}", step.id))?;
        }
    }
    let mut outputs = BTreeMap::new();
    for output in &step.outputs {
        let path = build.join(output);
        ensure!(path.is_file(), "step {} did not write {output}", step.id);
        outputs.insert(output.clone(), digest::file(&path)?);
        if cache && !cached {
            copy(&path, &cached_dir.join(output))?;
        }
    }
    Ok(StepReceipt {
        step: step.id.clone(),
        runner: stage.mode().name().into(),
        cached,
        inputs,
        outputs,
        adjustments: adjustments(step, &addons),
    })
}

/// The step's arguments, after the force-include flags that load its
/// prelude, which this writes into the build tree.
fn with_prelude(compiler: &Installed, stage: &Stage, step: &Step) -> Result<Vec<String>> {
    let mut args = step.args.clone();
    let build = stage.build();
    if let Some(prelude) = &step.prelude {
        ensure!(
            !compiler.tool.force_include.is_empty(),
            "{} declares no force-include flag, so preludes cannot be used",
            compiler.receipt.id
        );
        let relative = format!("prelude/{}.h", step.id);
        let path = build.join(&relative);
        fs::create_dir_all(path.parent().context("prelude path")?)?;
        fs::write(&path, prelude.render())?;
        // Compilers resolve a forced include like `#include "…"`, from the
        // source's directory, so the path is spelled from the build root.
        let spelled = if stage.mode().dos() {
            format!("${{BUILD}}\\prelude\\{}.h", step.id)
        } else {
            format!("${{BUILD}}/{relative}")
        };
        let mut flags: Vec<String> = compiler
            .tool
            .force_include
            .iter()
            .map(|flag| flag.replace("{}", &spelled))
            .collect();
        flags.append(&mut args);
        args = flags;
    }
    Ok(args)
}

/// What besides the compiler shaped the step's outputs, for its receipt.
fn adjustments(step: &Step, addons: &[Addon]) -> Vec<String> {
    let prelude = step
        .prelude
        .iter()
        .map(|prelude| format!("prelude {}", grind::describe(prelude)));
    let registers = step.registers.iter().map(|(symbol, registers)| {
        let renames: Vec<String> = registers
            .iter()
            .map(|(from, to)| format!("{from}->{to}"))
            .collect();
        format!("registers {symbol} {}", renames.join(" "))
    });
    let rewrites = addons.iter().zip(&step.rewrite).map(|(addon, rewrite)| {
        format!(
            "rewrite {} {} (sha256 {})",
            rewrite.addon,
            rewrite.args.join(" "),
            addon.tree
        )
    });
    prelude.chain(registers).chain(rewrites).collect()
}

/// Finds a step's program: a compiler command, `tool:command` from another
/// catalog tool, or a path inside the compiler.
fn resolve(
    catalog: &Catalog,
    compiler: &Installed,
    program: &str,
) -> Result<(PathBuf, bool, String)> {
    if let Some((id, name)) = program.split_once(':')
        && !id.is_empty()
        && !name.starts_with('\\')
    {
        let installed = tools::find(catalog, id)?
            .with_context(|| format!("{id} is not installed; run `rbit setup`"))?;
        return Ok((
            tools::command(&installed, Some(name))?,
            false,
            installed.receipt.tree,
        ));
    }
    Ok((
        tools::command(compiler, Some(program))?,
        true,
        compiler.receipt.tree.clone(),
    ))
}

/// The files the producer may see: every `source/` input plus every
/// header-like file, and one digest over the headers so a header edit
/// invalidates the cached steps that might include it.
///
/// # Errors
///
/// Fails when a step would read a target's reference binary.
pub(crate) fn sources(loaded: &Loaded) -> Result<(String, Vec<String>)> {
    let skip = [
        loaded.project.state.as_str(),
        ".git",
        "target",
        "node_modules",
    ];
    let wanted = &loaded.project.headers;
    let mut listing = String::new();
    let mut files: Vec<String> = Vec::new();
    for relative in digest::files(&loaded.root, &skip)? {
        let extension = relative
            .rsplit('.')
            .next()
            .unwrap_or_default()
            .to_lowercase();
        if wanted
            .iter()
            .any(|item| item.eq_ignore_ascii_case(&extension))
        {
            writeln!(
                listing,
                "{}  {relative}",
                digest::file(&loaded.root.join(&relative))?
            )?;
            files.push(relative);
        }
    }
    let references: Vec<&str> = loaded
        .project
        .targets
        .iter()
        .map(|target| target.reference.as_str())
        .collect();
    for step in &loaded.project.steps {
        for input in step
            .inputs
            .iter()
            .filter_map(|input| input.strip_prefix("source/"))
        {
            ensure!(
                !references.contains(&input),
                "step {} reads the reference {input}; producers may never see a reference",
                step.id
            );
            files.push(input.to_owned());
        }
    }
    files.sort();
    files.dedup();
    Ok((digest::bytes(listing.as_bytes()), files))
}

fn copy(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(from, to).with_context(|| format!("cannot copy {}", from.display()))?;
    Ok(())
}
