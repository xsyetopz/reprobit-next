//! `rbit space` and `rbit gc`: what reprobit holds on disk, and freeing it.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use serde_json::json;

use super::{Outcome, done};
use crate::catalog::Catalog;
use crate::project::{FILE, Loaded};
use crate::space::{self, Item, Scope, human};

/// The project in `dir`, when it has one; a directory without one is not an error,
/// because both commands also report the machine-wide home.
fn project(dir: &Path) -> Result<Option<Loaded>> {
    if dir.join(FILE).is_file() {
        Loaded::open(dir).map(Some)
    } else {
        Ok(None)
    }
}

fn survey(dir: &Path, keep_days: u64) -> Result<Vec<Item>> {
    let loaded = project(dir)?;
    let scope = Scope {
        project: loaded.as_ref(),
        keep: Duration::from_secs(keep_days.saturating_mul(86_400)),
    };
    space::survey(&Catalog::load()?, &scope)
}

/// Bytes of `items` that pass `keep`.
fn total(items: &[Item], keep: impl Fn(&Item) -> bool) -> u64 {
    items
        .iter()
        .filter(|item| keep(item))
        .map(|item| item.bytes)
        .sum()
}

fn reclaimable(item: &Item, images: bool) -> bool {
    item.reclaim.is_some() && (images == (item.kind == space::Kind::Image))
}

pub(super) fn space(dir: &Path, keep_days: u64) -> Result<Outcome> {
    let items = survey(dir, keep_days)?;
    let held = total(&items, |_| true);
    let files = total(&items, |item| reclaimable(item, false));
    let images = total(&items, |item| reclaimable(item, true));
    let mut text = String::new();
    for item in &items {
        writeln!(
            text,
            "{:>9}  {}{}",
            human(item.bytes),
            item.name,
            item.reclaim
                .as_ref()
                .map_or_default(|why| format!("  (reclaimable: {why})"))
        )?;
    }
    write!(
        text,
        "{} held, {} reclaimable with `rbit gc`",
        human(held),
        human(files)
    )?;
    if images > 0 {
        write!(text, ", {} more with `rbit gc --images`", human(images))?;
    }
    Ok(done(
        json!({ "items": items, "bytes": held, "reclaimable": files, "reclaimable_images": images }),
        text,
    ))
}

pub(super) fn gc(dir: &Path, keep_days: u64, images: bool, dry_run: bool) -> Result<Outcome> {
    let items = survey(dir, keep_days)?;
    let removed = if dry_run {
        items
            .iter()
            .filter(|item| reclaimable(item, false) || (images && reclaimable(item, true)))
            .cloned()
            .collect()
    } else {
        space::reclaim(&items, images)?
    };
    let freed = total(&removed, |_| true);
    let verb = if dry_run { "Would free" } else { "Freed" };
    let mut text = removed
        .iter()
        .map(|item| format!("{:>9}  {}", human(item.bytes), item.name))
        .collect::<Vec<_>>()
        .join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    write!(text, "{verb} {}", human(freed))?;
    Ok(done(
        json!({ "removed": removed, "freed": freed, "dry_run": dry_run }),
        text,
    ))
}
