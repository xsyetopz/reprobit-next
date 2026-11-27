//! `rbit locate`: where a library's functions sit in a linked binary.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::Outcome;
use crate::library::{self, Status};

pub(super) fn locate(binary: &Path, libraries: &[PathBuf], all: bool) -> Result<Outcome> {
    let read =
        |path: &Path| fs::read(path).with_context(|| format!("cannot read {}", path.display()));
    let image = read(binary)?;
    let mut objects = Vec::new();
    for path in libraries {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        objects.extend(library::objects(&name, &read(path)?)?);
    }
    let summary = library::locate(&image, &objects, all)?;
    let mut text = String::new();
    for member in &summary.members {
        if member.linked {
            writeln!(text, "linked     {}", member.name)?;
            continue;
        }
        let count = |status: Status| {
            member
                .functions
                .iter()
                .filter(|item| item.status == status)
                .count()
        };
        // One line per partly placed member: a member's absent functions, one per line,
        // made up most of a vendor archive's report.
        writeln!(
            text,
            "partial    {}: {} of {} found, {} absent",
            member.name,
            count(Status::Found),
            member.functions.len() + member.absent,
            count(Status::Absent) + member.absent
        )?;
        for item in &member.functions {
            match (item.status, item.places) {
                (Status::Found | Status::Absent, _) => {}
                (Status::Ambiguous, _) if !item.shared.is_empty() => writeln!(
                    text,
                    "ambiguous  {}: {} (shared with {})",
                    member.name,
                    item.name,
                    item.shared.join(", ")
                )?,
                (Status::Ambiguous, places) => writeln!(
                    text,
                    "ambiguous  {}: {} ({} places)",
                    member.name,
                    item.name,
                    places.unwrap_or_default()
                )?,
            }
        }
    }
    writeln!(
        text,
        "{}/{} members linked, {} with nothing placed; {} functions, {} bytes found",
        summary.linked,
        summary.members.len() + summary.unplaced,
        summary.unplaced,
        summary.found,
        summary.found_bytes
    )?;
    Ok(Outcome {
        ok: true,
        json: serde_json::to_value(&summary)?,
        text,
    })
}
