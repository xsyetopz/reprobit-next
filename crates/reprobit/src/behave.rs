//! `rbit behave`: behaviour checks for recompilation. A recompiled target is
//! built by a different compiler, so its bytes differ from the original by
//! design; what can still be compared is what both binaries do. Each check
//! runs one declared command against the reference and against the artifact
//! and compares the results it names.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::build;
use crate::digest;
use crate::project::{Check, Loaded};

/// Text kept per stream in the report; the digest covers all of it.
const SHOWN: usize = 4096;

/// What one binary did.
#[derive(Debug, Serialize)]
pub(crate) struct Run {
    /// Exit code; `None` when a signal or the timeout ended it.
    pub(crate) exit: Option<i32>,
    pub(crate) timed_out: bool,
    pub(crate) stdout_sha256: String,
    pub(crate) stderr_sha256: String,
    /// The start of each stream, for reading a difference.
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

/// One check's two runs.
#[derive(Debug, Serialize)]
pub(crate) struct Outcome {
    pub(crate) id: String,
    pub(crate) target: String,
    pub(crate) agree: bool,
    /// The compared results that differ.
    pub(crate) differs: Vec<String>,
    pub(crate) reference: Run,
    pub(crate) artifact: Run,
}

/// Builds incrementally, then runs every check, or only `only` when given.
pub(crate) fn behave(loaded: &Loaded, only: &[String]) -> Result<Vec<Outcome>> {
    let checks: Vec<&Check> = loaded
        .project
        .checks
        .iter()
        .filter(|check| only.is_empty() || only.contains(&check.id))
        .collect();
    if checks.is_empty() {
        bail!("no [[check]] to run; add one to reprobit.toml, as docs/project.md shows");
    }
    let built = build::run(loaded, &loaded.state().join("stage"), true)?;
    let mut outcomes = Vec::new();
    for check in checks {
        let target = loaded
            .project
            .targets
            .iter()
            .find(|target| target.id == check.target)
            .context("the check's target is gone")?;
        let name = Path::new(&target.reference)
            .file_name()
            .context("the reference has no file name")?;
        let scratch = loaded.state().join("behave").join(&check.id);
        let reference = run(
            loaded,
            check,
            &loaded.root.join(&target.reference),
            &scratch,
            name,
        )?;
        let artifact = run(
            loaded,
            check,
            &built.stage.build().join(&target.artifact),
            &scratch,
            name,
        )?;
        let differs: Vec<String> = check
            .compare
            .iter()
            .filter(|field| match field.as_str() {
                "exit" => reference.exit != artifact.exit,
                "stdout" => reference.stdout_sha256 != artifact.stdout_sha256,
                _ => reference.stderr_sha256 != artifact.stderr_sha256,
            })
            .cloned()
            .collect();
        outcomes.push(Outcome {
            id: check.id.clone(),
            target: check.target.clone(),
            agree: differs.is_empty() && !reference.timed_out && !artifact.timed_out,
            differs,
            reference,
            artifact,
        });
    }
    Ok(outcomes)
}

/// Runs `check` on a copy of `binary` named `name` in a fresh `directory`.
/// Both sides run in the same directory, so a program that prints its own
/// path prints the same text.
fn run(
    loaded: &Loaded,
    check: &Check,
    binary: &Path,
    directory: &Path,
    name: &std::ffi::OsStr,
) -> Result<Run> {
    if directory.exists() {
        fs::remove_dir_all(directory)?;
    }
    fs::create_dir_all(directory)?;
    let copy: PathBuf = directory.join(name);
    fs::copy(binary, &copy).with_context(|| format!("cannot read {}", binary.display()))?;
    let spelled = copy.to_string_lossy();
    let mut words = check
        .run
        .iter()
        .map(|word| word.replace("${BINARY}", &spelled));
    let program = words.next().context("the check's run is empty")?;
    let stdout = directory.join(".stdout");
    let stderr = directory.join(".stderr");
    let stdin = match &check.stdin {
        Some(file) => Stdio::from(
            fs::File::open(loaded.root.join(file))
                .with_context(|| format!("cannot open stdin file {file}"))?,
        ),
        None => Stdio::null(),
    };
    let mut child = Command::new(&program)
        .args(words)
        .current_dir(directory)
        .stdin(stdin)
        .stdout(fs::File::create(&stdout)?)
        .stderr(fs::File::create(&stderr)?)
        .spawn()
        .with_context(|| format!("cannot start {program}"))?;
    let deadline = Instant::now() + Duration::from_secs(check.timeout);
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait()? {
            break (Some(status), false);
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            break (None, true);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let read = |path: &Path| fs::read(path).unwrap_or_default();
    let (out, err) = (read(&stdout), read(&stderr));
    let shown =
        |bytes: &[u8]| String::from_utf8_lossy(&bytes[..bytes.len().min(SHOWN)]).into_owned();
    Ok(Run {
        exit: status.and_then(|status| status.code()),
        timed_out,
        stdout_sha256: digest::bytes(&out),
        stderr_sha256: digest::bytes(&err),
        stdout: shown(&out),
        stderr: shown(&err),
    })
}
