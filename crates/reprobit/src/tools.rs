//! Installing, finding, and removing catalog tools. Each install is one
//! directory under [`home::tools`] with a receipt recording the content
//! digest, so a project lock can prove two machines run identical bytes.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use crate::catalog::{Catalog, Get, Tool};
use crate::{digest, fetch, home, ui};

/// The receipt file written into every install.
pub(crate) const RECEIPT: &str = ".reprobit-tool.toml";

/// What an install recorded about itself.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Receipt {
    /// Catalog id.
    pub(crate) id: String,
    /// Catalog version.
    pub(crate) version: String,
    /// [`digest::tree`] of the installed files, receipt excluded.
    pub(crate) tree: String,
    /// Resolved pins, such as an OCI manifest digest for a `latest` tag.
    #[serde(default)]
    pub(crate) pins: Vec<String>,
    /// Where the executables run, when the catalog says `detect`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) runs: Option<String>,
}

/// An installed tool: its catalog entry, location, and receipt.
#[derive(Debug, Clone)]
pub(crate) struct Installed {
    pub(crate) tool: Tool,
    pub(crate) root: PathBuf,
    pub(crate) receipt: Receipt,
}

fn version_of(tool: &Tool) -> &str {
    if tool.version.is_empty() {
        "latest"
    } else {
        &tool.version
    }
}

/// The install directory for `id` at its catalog version.
pub(crate) fn location(id: &str, tool: &Tool) -> PathBuf {
    let mut path = home::tools();
    path.extend(id.split('/'));
    path.join(version_of(tool))
}

/// The installed copy of `id`, if any.
pub(crate) fn find(catalog: &Catalog, id: &str) -> Result<Option<Installed>> {
    let tool = catalog.get(id)?;
    let root = location(id, &tool);
    let path = root.join(RECEIPT);
    if !path.is_file() {
        return Ok(None);
    }
    let receipt: Receipt = toml::from_str(&fs::read_to_string(&path)?)
        .with_context(|| format!("{} is damaged", path.display()))?;
    let mut tool = tool;
    if let Some(runs) = &receipt.runs {
        tool.runs.clone_from(runs);
    }
    Ok(Some(Installed {
        tool,
        root,
        receipt,
    }))
}

/// Installs `id` unless it is already present, and returns the install.
pub(crate) fn install(catalog: &Catalog, id: &str) -> Result<Installed> {
    if let Some(found) = find(catalog, id)? {
        return Ok(found);
    }
    let tool = catalog.get(id)?;
    let fetches = tool.fetches();
    ensure!(
        !fetches.is_empty(),
        "{id} has no download for {}; see {}",
        home::host(),
        tool.homepage
    );
    for command in &tool.needs {
        ensure!(
            on_path(command).is_some(),
            "{id} needs `{command}` on PATH; install it first"
        );
    }
    let root = location(id, &tool);
    // Virtual environments and npm prefixes embed their own path, so those
    // tools are built in place; everything else is staged and renamed in, so
    // a failed install never leaves a half-written tool behind.
    let in_place = fetches
        .iter()
        .any(|get| get.pypi.is_some() || get.npm.is_some());
    let staging = if in_place {
        root.clone()
    } else {
        root.with_extension("partial")
    };
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;
    let result = populate(&tool, &fetches, &staging);
    let pins = match result {
        Ok(pins) => pins,
        Err(error) => {
            if let Err(cleanup) = fs::remove_dir_all(&staging) {
                ui::note(&format!("cannot remove {}: {cleanup}", staging.display()));
            }
            return Err(error.context(format!("installing {id} failed")));
        }
    };
    let runs = if tool.runs == "detect" {
        Some(detect(&staging)?)
    } else {
        None
    };
    let receipt = Receipt {
        id: id.into(),
        version: version_of(&tool).into(),
        tree: digest::tree(&staging, &[RECEIPT])?,
        pins,
        runs,
    };
    fs::write(staging.join(RECEIPT), toml::to_string(&receipt)?)?;
    if !in_place {
        if root.exists() {
            fs::remove_dir_all(&root)?;
        }
        fs::rename(&staging, &root)?;
    }
    ui::note(&format!("Installed {id} into {}", root.display()));
    Ok(Installed {
        tool,
        root,
        receipt,
    })
}

fn populate(tool: &Tool, fetches: &[&Get], into: &Path) -> Result<Vec<String>> {
    let mut pins = Vec::new();
    for get in fetches {
        let scratch = into.with_extension("fetch");
        if scratch.exists() {
            fs::remove_dir_all(&scratch)?;
        }
        let picking = !get.pick.is_empty();
        let target = if picking { scratch.as_path() } else { into };
        if let Some(url) = &get.url {
            let archive = fetch::download(url, get.sha256.as_deref(), &[])?;
            let name = get.save_as.clone().unwrap_or_else(|| {
                url.rsplit(['/', '\\'])
                    .next()
                    .unwrap_or("download")
                    .to_owned()
            });
            fetch::unpack(&archive, target, &get.strip, &name)?;
        } else if let Some(url) = &get.git {
            let rev = get
                .rev
                .as_deref()
                .context("a git fetch needs `rev`, the full commit")?;
            let paths: Vec<&str> = get.pick.keys().map(String::as_str).collect();
            fetch::git(url, rev, &paths, target)?;
        } else if let Some(image) = &get.oci {
            pins.push(format!(
                "{image}@{}",
                fetch::oci(
                    image,
                    get.reference.as_deref().unwrap_or("latest"),
                    &get.strip,
                    target
                )?
            ));
        } else if let Some(requirement) = &get.pypi {
            fetch::pypi(requirement, into)?;
        } else if let Some(package) = &get.npm {
            fetch::npm(package, into)?;
        } else {
            bail!("a fetch needs one of url, git, oci, pypi, or npm");
        }
        for (from, to) in &get.pick {
            let source = scratch.join(from);
            let destination = into.join(if to.is_empty() { from } else { to });
            copy(&source, &destination).with_context(|| format!("cannot pick {from}"))?;
        }
        if scratch.exists() {
            fs::remove_dir_all(&scratch)?;
        }
    }
    for patch in &tool.patch {
        let path = into.join(&patch.file);
        let mut bytes = fs::read(&path).with_context(|| format!("cannot patch {}", patch.file))?;
        let (from, to) = (hex::decode(&patch.from)?, hex::decode(&patch.to)?);
        ensure!(
            from.len() == to.len(),
            "a patch to {} changes its length",
            patch.file
        );
        let start = usize::try_from(patch.offset)?;
        let window = bytes
            .get_mut(start..start + from.len())
            .context("a patch offset is past the end of the file")?;
        ensure!(
            *window == from[..],
            "{} at {:#x} is not the expected preimage",
            patch.file,
            patch.offset
        );
        window.copy_from_slice(&to);
        fs::write(&path, bytes)?;
    }
    for (relative, expected) in &tool.verify {
        let actual = digest::file(&into.join(relative))?;
        ensure!(
            actual == *expected,
            "{relative} has SHA-256 {actual}, but the catalog pins {expected}"
        );
    }
    Ok(pins)
}

fn copy(source: &Path, destination: &Path) -> Result<()> {
    if source.is_dir() {
        for relative in digest::files(source, &[])? {
            copy(&source.join(&relative), &destination.join(&relative))?;
        }
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
}

/// Removes the install of `id`. Returns whether anything was there.
pub(crate) fn remove(catalog: &Catalog, id: &str) -> Result<bool> {
    let mut path = home::tools();
    path.extend(id.split('/'));
    let known = catalog.get(id).is_ok();
    ensure!(known || path.exists(), "no tool named {id}");
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(&path)?;
    Ok(true)
}

/// The path of command `name` (or the tool's only command) inside `installed`.
pub(crate) fn command(installed: &Installed, name: Option<&str>) -> Result<PathBuf> {
    let bin = &installed.tool.bin;
    let relative = match name {
        Some(name) => bin.get(name).cloned().unwrap_or_else(|| name.to_owned()),
        None if bin.len() == 1 => bin.values().next().cloned().unwrap_or_default(),
        None => bail!(
            "{} has several commands; name one of: {}",
            installed.receipt.id,
            bin.keys().cloned().collect::<Vec<_>>().join(", ")
        ),
    };
    let path = installed.root.join(&relative);
    if path.is_file() {
        return Ok(path);
    }
    // One `bin` entry serves every host, so `ruffle` also finds `ruffle.exe`.
    if cfg!(windows) && path.with_extension("exe").is_file() {
        return Ok(path.with_extension("exe"));
    }
    // Archives built on Windows often differ in case from the catalog, and a
    // macOS app must stay inside its signed bundle, so `ruffle` also finds
    // `Ruffle.app/Contents/MacOS/ruffle`.
    let wanted = relative.to_lowercase();
    let files = digest::files(&installed.root, &[RECEIPT])?;
    let exact = files
        .iter()
        .find(|candidate| candidate.to_lowercase() == wanted);
    let nested = || {
        files
            .iter()
            .find(|candidate| candidate.to_lowercase().ends_with(&format!("/{wanted}")))
    };
    exact
        .or_else(nested)
        .map(|found| installed.root.join(found))
        .with_context(|| format!("{} has no command {relative}", installed.receipt.id))
}

/// Where an install's executables run, read from their file formats: a
/// Mach-O anywhere means a native macOS build, any PE means Win32 programs
/// (helpers beside them are usually scripts), and ELF alone means Linux.
fn detect(root: &Path) -> Result<String> {
    let (mut macho, mut pe, mut i386, mut amd64) = (false, false, false, false);
    for relative in digest::files(root, &[])? {
        let mut header = [0u8; 20];
        if std::io::Read::read_exact(&mut fs::File::open(root.join(&relative))?, &mut header)
            .is_err()
        {
            continue;
        }
        match header {
            [0xcf | 0xce, 0xfa, 0xed, 0xfe, ..] | [0xca, 0xfe, 0xba, 0xbe, ..] => macho = true,
            [b'M', b'Z', ..] => pe = true,
            // ELF e_machine 3 is i386.
            [0x7f, b'E', b'L', b'F', ..] if header[18] == 3 => i386 = true,
            [0x7f, b'E', b'L', b'F', ..] => amd64 = true,
            _ => {}
        }
    }
    Ok(match (macho, pe, amd64, i386) {
        (true, ..) => "host",
        (_, true, ..) => "windows",
        (_, _, true, _) => "linux-x86_64",
        (.., true) => "linux-i386",
        _ => "host",
    }
    .into())
}

/// A process that starts the command at `path`; Java archives run through
/// `java -jar`.
pub(crate) fn launch(path: &Path) -> std::process::Command {
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
    {
        let mut command = std::process::Command::new("java");
        command.arg("-jar").arg(path);
        command
    } else {
        std::process::Command::new(path)
    }
}

/// The first `name` on `PATH`, like `command -v`.
pub(crate) fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let suffixes: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };
    std::env::split_paths(&path)
        .flat_map(|directory| {
            suffixes
                .iter()
                .map(move |suffix| directory.join(format!("{name}{suffix}")))
        })
        .find(|candidate| candidate.is_file())
}
