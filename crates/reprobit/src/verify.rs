//! Verification: a cold build from declared inputs only, then a literal
//! comparison of every target with its reference. This is the only path that
//! certifies a match, and the report it writes carries the whole evidence.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::binary::{self, Difference};
use crate::build::{self, StepReceipt};
use crate::digest;
use crate::project::{Loaded, Target};

/// The trust report. `reprobit report --json` and the MCP server return the same shape.
#[derive(Debug, Serialize)]
pub(crate) struct Report {
    /// Project name.
    pub(crate) project: String,
    /// `true` when every target is byte-identical.
    pub(crate) identical: bool,
    /// The compiler that produced the build.
    pub(crate) compiler: Compiler,
    /// How the compiler ran: `native`, `wine`, `wibo`, `docker`, or `subst`.
    pub(crate) runner: String,
    /// Every step's input and output digests, in run order.
    pub(crate) steps: Vec<StepReceipt>,
    /// One comparison per target.
    pub(crate) targets: Vec<Comparison>,
}

/// The compiler as installed.
#[derive(Debug, Serialize)]
pub(crate) struct Compiler {
    pub(crate) id: String,
    pub(crate) version: String,
    /// Content digest of the install; `reprobit.lock` pins the same value.
    pub(crate) tree: String,
}

/// One target compared with its reference.
#[derive(Debug, Serialize)]
pub(crate) struct Comparison {
    pub(crate) id: String,
    pub(crate) identical: bool,
    pub(crate) size: usize,
    pub(crate) reference_size: usize,
    /// SHA-256 of the rebuilt file as the linker wrote it.
    pub(crate) artifact_sha256: String,
    /// SHA-256 of the reference as it is on disk.
    pub(crate) reference_sha256: String,
    /// Normalizations applied to both sides, and the fields they cleared.
    pub(crate) normalized: Vec<String>,
    pub(crate) normalized_fields: usize,
    /// The first runs of differing bytes, after normalization.
    pub(crate) differences: Vec<Difference>,
}

/// Builds from scratch in `<state>/verify` and compares every target.
pub(crate) fn verify(loaded: &Loaded) -> Result<Report> {
    let stage = loaded.state().join("verify");
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    let built = build::run(loaded, &stage, false)?;
    let targets = compare(loaded, &built.stage.build())?;
    let report = Report {
        project: loaded.project.name.clone(),
        identical: targets.iter().all(|target| target.identical),
        compiler: Compiler {
            id: built.compiler.receipt.id.clone(),
            version: built.compiler.receipt.version.clone(),
            tree: built.compiler.receipt.tree.clone(),
        },
        runner: built.stage.mode().name().into(),
        steps: built.receipts,
        targets,
    };
    save(loaded, &report)?;
    Ok(report)
}

/// Compares each target in `build` with its reference.
pub(crate) fn compare(loaded: &Loaded, build: &Path) -> Result<Vec<Comparison>> {
    loaded
        .project
        .targets
        .iter()
        .map(|target| compare_one(loaded, target, build))
        .collect()
}

fn compare_one(loaded: &Loaded, target: &Target, build: &Path) -> Result<Comparison> {
    let reference_path = loaded.root.join(&target.reference);
    let mut reference = fs::read(&reference_path).with_context(|| {
        format!(
            "place the original at {} (it is never committed or redistributed)",
            target.reference
        )
    })?;
    let reference_sha256 = digest::bytes(&reference);
    let mut normalized_fields = binary::normalize(&mut reference, &target.normalize)?;
    check_reference(target, &reference)?;
    let mut artifact = fs::read(build.join(&target.artifact))
        .with_context(|| format!("target {} was not built", target.id))?;
    let artifact_sha256 = digest::bytes(&artifact);
    normalized_fields += binary::normalize(&mut artifact, &target.normalize)?;
    Ok(Comparison {
        id: target.id.clone(),
        identical: artifact == reference,
        size: artifact.len(),
        reference_size: reference.len(),
        artifact_sha256,
        reference_sha256,
        normalized: target.normalize.clone(),
        normalized_fields,
        differences: binary::differences(&artifact, &reference, 16),
    })
}

/// Checks a reference, already normalized, against the target's pin.
pub(crate) fn check_reference(target: &Target, normalized: &[u8]) -> Result<()> {
    if let Some(expected) = &target.sha256 {
        let actual = digest::bytes(normalized);
        ensure!(
            *expected == actual,
            "{} has SHA-256 {actual} after normalization, but the project expects {expected}; is it the right original?",
            target.reference
        );
    }
    Ok(())
}

/// Where reports are written.
pub(crate) fn report_dir(loaded: &Loaded) -> PathBuf {
    loaded.state().join("reports")
}

fn save(loaded: &Loaded, report: &Report) -> Result<()> {
    let directory = report_dir(loaded);
    fs::create_dir_all(&directory)?;
    fs::write(
        directory.join("report.json"),
        serde_json::to_string_pretty(report)?,
    )?;
    fs::write(directory.join("report.html"), html(report)?)?;
    Ok(())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A self-contained page with the same facts as the JSON report.
fn html(report: &Report) -> Result<String> {
    let verdict = if report.identical {
        "Byte-identical"
    } else {
        "Different"
    };
    let mut rows = String::new();
    for target in &report.targets {
        let differences: Vec<String> = target
            .differences
            .iter()
            .map(|item| {
                format!(
                    "{:#x}+{} {}",
                    item.offset,
                    item.length,
                    item.section.as_deref().unwrap_or("")
                )
            })
            .collect();
        writeln!(
            rows,
            "<tr><td>{}</td><td>{}</td><td>{} / {}</td><td><code>{}</code></td><td><code>{}</code></td><td>{} ({} fields)</td><td>{}</td></tr>",
            escape(&target.id),
            if target.identical {
                "identical"
            } else {
                "different"
            },
            target.size,
            target.reference_size,
            target.artifact_sha256,
            target.reference_sha256,
            escape(&target.normalized.join(", ")),
            target.normalized_fields,
            escape(&differences.join("<br>")),
        )?;
    }
    let mut steps = String::new();
    for step in &report.steps {
        let list = |map: &std::collections::BTreeMap<String, String>| {
            map.iter()
                .map(|(path, sha)| format!("{} <code>{}</code>", escape(path), &sha[..16]))
                .collect::<Vec<_>>()
                .join("<br>")
        };
        writeln!(
            steps,
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape(&step.step),
            list(&step.inputs),
            list(&step.outputs)
        )?;
    }
    Ok(format!(
        "<!doctype html><meta charset=utf-8><title>{name}: {verdict}</title>\
<style>body{{font:14px system-ui;margin:2rem;max-width:80rem}}td,th{{border-bottom:1px solid #ddd;padding:.3rem;text-align:left;vertical-align:top}}table{{border-collapse:collapse}}code{{font-size:12px}}</style>\
<h1>{name}: {verdict}</h1><p>Built from scratch by <b>{compiler} {version}</b> (content <code>{tree}</code>) under <b>{runner}</b>. \
The producer saw only the declared inputs below; references were read only by the comparison.</p>\
<h2>Targets</h2><table><tr><th>Target</th><th>Result</th><th>Size / reference</th><th>Rebuilt SHA-256</th><th>Reference SHA-256</th><th>Normalized</th><th>First differences</th></tr>{rows}</table>\
<h2>Steps</h2><table><tr><th>Step</th><th>Inputs</th><th>Outputs</th></tr>{steps}</table>\n",
        name = escape(&report.project),
        compiler = escape(&report.compiler.id),
        version = escape(&report.compiler.version),
        tree = report.compiler.tree,
        runner = report.runner,
    ))
}
