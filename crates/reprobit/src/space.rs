//! What reprobit keeps on disk, and which of it can go. A laptop with 128 or
//! 256 GB of storage has little room after the operating system, so every
//! byte reprobit holds is listed with whether it can be fetched or rebuilt.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::catalog::Catalog;
use crate::home;
use crate::project::{Loaded, SCRATCH};
use crate::runner::WINE_IMAGE;
use crate::tools::{self, Receipt};

/// The marker a step-cache hit refreshes, so age means time since last use.
pub(crate) const USED: &str = ".used";

/// What one item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Kind {
    /// Archives kept after unpacking.
    Downloads,
    /// One installed tool version.
    Tool,
    /// A container image the docker runner built.
    Image,
    /// A project's build, verify, grind, report, and log directories.
    Scratch,
    /// One entry of a project's step cache.
    Cache,
}

/// One thing on disk.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Item {
    pub(crate) kind: Kind,
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<PathBuf>,
    pub(crate) bytes: u64,
    /// Why `rbit gc` may remove it; `None` keeps it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reclaim: Option<String>,
}

/// What a survey looks at.
pub(crate) struct Scope<'a> {
    pub(crate) project: Option<&'a Loaded>,
    /// Step-cache entries unused for longer than this are reclaimable.
    pub(crate) keep: Duration,
}

/// Lists everything reprobit holds for this user, and for `scope.project`.
pub(crate) fn survey(catalog: &Catalog, scope: &Scope<'_>) -> Result<Vec<Item>> {
    let mut items = Vec::new();
    let downloads = home::downloads();
    if downloads.exists() {
        items.push(Item {
            kind: Kind::Downloads,
            name: "downloads".into(),
            bytes: size(&downloads)?,
            path: Some(downloads),
            reclaim: Some("an install fetches and checks it again by digest".into()),
        });
    }
    for (root, receipt) in installs(&home::tools())? {
        let reclaim = match catalog.get(&receipt.id) {
            Err(_) => Some("no catalog lists this tool".into()),
            Ok(tool) if tools::location(&receipt.id, &tool) != root => {
                Some(format!("the catalog now pins {}", tool.version))
            }
            Ok(_) => None,
        };
        items.push(Item {
            kind: Kind::Tool,
            name: format!("{} {}", receipt.id, receipt.version),
            bytes: size(&root)?,
            path: Some(root),
            reclaim,
        });
    }
    items.extend(images());
    if let Some(project) = scope.project {
        items.extend(project_items(project, scope.keep)?);
    }
    items.sort_by_key(|item| std::cmp::Reverse(item.bytes));
    Ok(items)
}

/// Removes each reclaimable item, images only when `images` is set because
/// the container store is shared with the user's other work. Returns what went.
pub(crate) fn reclaim(items: &[Item], images: bool) -> Result<Vec<Item>> {
    let mut removed = Vec::new();
    for item in items.iter().filter(|item| item.reclaim.is_some()) {
        match (&item.path, item.kind) {
            (Some(path), _) => {
                fs::remove_dir_all(path)
                    .with_context(|| format!("cannot remove {}", path.display()))?;
                if item.kind == Kind::Tool {
                    prune_empty(path.parent(), &home::tools());
                }
            }
            (None, Kind::Image) if images => {
                let Some(cli) = container_cli() else { continue };
                let status = Command::new(cli)
                    .args(["image", "rm", &item.name])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()?;
                if !status.success() {
                    continue;
                }
            }
            (None, _) => continue,
        }
        removed.push(item.clone());
    }
    Ok(removed)
}

fn project_items(project: &Loaded, keep: Duration) -> Result<Vec<Item>> {
    let state = project.state();
    let mut items = Vec::new();
    for name in SCRATCH {
        let path = state.join(name);
        if path.exists() {
            items.push(Item {
                kind: Kind::Scratch,
                name: format!("{}/{name}", project.project.state),
                bytes: size(&path)?,
                path: Some(path),
                reclaim: Some("the next build or verify writes it again".into()),
            });
        }
    }
    let cache = state.join("cache");
    if cache.is_dir() {
        let now = SystemTime::now();
        for entry in fs::read_dir(&cache)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            let used = fs::metadata(path.join(USED))
                .or_else(|_| fs::metadata(&path))?
                .modified()?;
            let idle = now.duration_since(used).unwrap_or_default();
            items.push(Item {
                kind: Kind::Cache,
                name: format!(
                    "{}/cache/{}",
                    project.project.state,
                    path.file_name().unwrap_or_default().to_string_lossy()
                ),
                bytes: size(&path)?,
                reclaim: (idle > keep).then(|| {
                    format!(
                        "unused for {} days; the step runs again",
                        idle.as_secs() / 86_400
                    )
                }),
                path: Some(path),
            });
        }
    }
    Ok(items)
}

/// Every install under `root`: a directory holding a receipt. Ids may nest
/// (`decompme/ps2/ee-gcc2.96`), so the walk descends until it finds one.
fn installs(root: &Path) -> Result<Vec<(PathBuf, Receipt)>> {
    let mut found = Vec::new();
    if !root.is_dir() {
        return Ok(found);
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let receipt = dir.join(tools::RECEIPT);
        if receipt.is_file() {
            if let Ok(parsed) = toml::from_str::<Receipt>(&fs::read_to_string(&receipt)?) {
                found.push((dir, parsed));
            }
            continue;
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                pending.push(entry.path());
            }
        }
    }
    Ok(found)
}

/// Superseded runner images are reclaimable; the current one is rebuilt only
/// at a cost of minutes, so it stays.
fn images() -> Vec<Item> {
    let Some(cli) = container_cli() else {
        return Vec::new();
    };
    let repository = WINE_IMAGE.split(':').next().unwrap_or(WINE_IMAGE);
    let Ok(output) = Command::new(&cli)
        .args([
            "image",
            "ls",
            repository,
            "--format",
            "{{.Repository}}:{{.Tag}}",
        ])
        .stderr(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|name| !name.is_empty() && !name.ends_with(":<none>"))
        .map(|name| Item {
            kind: Kind::Image,
            name: name.to_owned(),
            path: None,
            bytes: image_bytes(&cli, name),
            reclaim: (name != WINE_IMAGE).then(|| format!("{WINE_IMAGE} replaced it")),
        })
        .collect()
}

fn image_bytes(cli: &Path, name: &str) -> u64 {
    Command::new(cli)
        .args(["image", "inspect", name, "--format", "{{.Size}}"])
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|output| String::from_utf8_lossy(&output.stdout).trim().parse().ok())
        .unwrap_or(0)
}

fn container_cli() -> Option<PathBuf> {
    tools::on_path("docker").or_else(|| tools::on_path("podman"))
}

/// Removes empty parents of a removed install up to, not including, `stop`.
fn prune_empty(mut dir: Option<&Path>, stop: &Path) {
    while let Some(path) = dir {
        if path == stop || fs::remove_dir(path).is_err() {
            break;
        }
        dir = path.parent();
    }
}

/// Bytes under `path`, symlinks counted as links, not followed.
pub(crate) fn size(path: &Path) -> Result<u64> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() {
        return Ok(meta.len());
    }
    let mut total = 0;
    for entry in fs::read_dir(path)? {
        total += size(&entry?.path())?;
    }
    Ok(total)
}

/// `bytes` as a short human size, such as `1.9 GB`.
pub(crate) fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    #[expect(
        clippy::cast_precision_loss,
        reason = "a display size needs no exact bytes"
    )]
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
