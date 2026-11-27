//! `rbit import`: compile steps from a `compile_commands.json`, the format
//! `CMake` (`CMAKE_EXPORT_COMPILE_COMMANDS`), Ninja (`-t compdb`), Meson, and
//! Bear all write, so one importer covers every build system that emits it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

use crate::project::Step;

/// One entry of a compilation database.
#[derive(Debug, Deserialize)]
struct Entry {
    directory: PathBuf,
    file: PathBuf,
    #[serde(default)]
    arguments: Vec<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    output: Option<PathBuf>,
}

/// Turns each entry of `database` into a step whose paths are portable:
/// the source becomes `${SOURCE}/…` relative to `root`, and the object
/// becomes a file name in the build root.
pub(crate) fn compile_commands(database: &Path, root: &Path) -> Result<Vec<Step>> {
    let text = std::fs::read_to_string(database)
        .with_context(|| format!("cannot read {}", database.display()))?;
    let entries: Vec<Entry> = serde_json::from_str(&text)
        .with_context(|| format!("{} is not a compilation database", database.display()))?;
    let root = root.canonicalize()?;
    entries
        .into_iter()
        .map(|entry| step(entry, &root))
        .collect()
}

fn step(entry: Entry, root: &Path) -> Result<Step> {
    let file = absolute(&entry.directory, &entry.file);
    let relative = file
        .strip_prefix(root)
        .with_context(|| format!("{} is outside {}", file.display(), root.display()))?
        .to_string_lossy()
        .replace('\\', "/");
    // `command` is one shell string; splitting on spaces covers the
    // generated databases, which quote only paths that contain spaces.
    let mut argv = if entry.arguments.is_empty() {
        entry
            .command
            .as_deref()
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    } else {
        entry.arguments
    };
    ensure!(!argv.is_empty(), "the entry for {relative} has no command");
    let program = Path::new(&argv.remove(0))
        .file_stem()
        .map_or_default(|stem| stem.to_string_lossy().into_owned());
    let output = entry.output.as_ref().map_or_else(
        || Path::new(&relative).with_extension("o"),
        |output| absolute(&entry.directory, output),
    );
    let object = output
        .file_name()
        .map_or_default(|name| name.to_string_lossy().into_owned());
    let source = format!("${{SOURCE}}/{relative}");
    let spelled = [
        (file.to_string_lossy().into_owned(), source.as_str()),
        (entry.file.to_string_lossy().into_owned(), source.as_str()),
        (output.to_string_lossy().into_owned(), object.as_str()),
        (
            entry
                .output
                .as_ref()
                .map_or_default(|raw| raw.to_string_lossy().into_owned()),
            object.as_str(),
        ),
    ];
    let prefix = format!("{}/", root.display());
    // A path matches a whole argument, or the end of a flag such as
    // `/Fo<path>` when it is absolute; short relative names could otherwise
    // match inside unrelated arguments.
    let args = argv
        .into_iter()
        .map(|arg| {
            let found = spelled.iter().find(|(from, _)| {
                !from.is_empty()
                    && (arg == *from
                        || (Path::new(from).is_absolute() && arg.ends_with(from.as_str())))
            });
            match found {
                Some((from, to)) => arg.replace(from.as_str(), to),
                None => arg.replace(&prefix, "${SOURCE}/"),
            }
        })
        .collect();
    let stem = Path::new(&relative)
        .file_stem()
        .map_or_default(|stem| stem.to_string_lossy().into_owned());
    Ok(Step {
        id: format!("compile.{stem}"),
        program,
        args,
        inputs: vec![format!("source/{relative}")],
        outputs: vec![object],
        env: BTreeMap::new(),
        prelude: None,
        registers: BTreeMap::new(),
        rewrite: Vec::new(),
        reference_object: None,
    })
}

fn absolute(directory: &Path, path: &Path) -> PathBuf {
    let joined = directory.join(path);
    joined.canonicalize().unwrap_or(joined)
}
