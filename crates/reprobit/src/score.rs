//! `rbit score`: grades each function of a built object against the same
//! symbol in a reference object. Decompilation projects track progress as
//! the share of functions that match, so the summary gives that count, and
//! a grade per function says how far each one is from it.

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::binary::{self, Function};

/// How one function compares with its reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Grade {
    /// Same bytes and the same relocations to the same symbols.
    Exact,
    /// Same instructions; only relocation targets or the bytes the linker
    /// patches differ, such as a string literal's compiler-generated name.
    Reloc,
    /// The instructions differ.
    Different,
    /// The built object does not define the symbol.
    Missing,
}

/// One function's grade.
#[derive(Debug, Serialize)]
pub(crate) struct Scored {
    /// The compile step the function came from, in a project-wide score.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) unit: Option<String>,
    pub(crate) name: String,
    pub(crate) grade: Grade,
    pub(crate) size: Option<usize>,
    pub(crate) reference_size: usize,
    /// Share of byte positions that agree, from 0 to 100, after the fields
    /// either side relocates are cleared.
    pub(crate) matching_percent: f64,
}

/// Every graded function and the totals.
#[derive(Debug, Serialize)]
pub(crate) struct Summary {
    pub(crate) functions: usize,
    pub(crate) exact: usize,
    pub(crate) reloc: usize,
    pub(crate) different: usize,
    pub(crate) missing: usize,
    /// `exact + reloc` as a share of `functions`, from 0 to 100.
    pub(crate) matched_percent: f64,
    pub(crate) symbols: Vec<Scored>,
}

/// Grades `symbols`, or every function the reference defines when empty.
pub(crate) fn score(reference: &[u8], built: &[u8], symbols: &[String]) -> Result<Summary> {
    let names = if symbols.is_empty() {
        binary::functions(reference).context("in the reference object")?
    } else {
        symbols.to_vec()
    };
    ensure!(
        !names.is_empty(),
        "the reference object defines no functions"
    );
    let mut scored = Vec::with_capacity(names.len());
    for name in names {
        let wanted = binary::function(reference, &name).context("in the reference object")?;
        let reference_size = wanted.bytes.len();
        scored.push(match binary::function(built, &name) {
            Ok(got) => {
                let (grade, matching_percent) = grade(&wanted, &got);
                Scored {
                    unit: None,
                    name,
                    grade,
                    size: Some(got.bytes.len()),
                    reference_size,
                    matching_percent,
                }
            }
            Err(_) => Scored {
                unit: None,
                name,
                grade: Grade::Missing,
                size: None,
                reference_size,
                matching_percent: 0.0,
            },
        });
    }
    Ok(total(scored))
}

/// Grades every function of `reference` as missing, for a unit not built yet.
pub(crate) fn unbuilt(reference: &[u8]) -> Result<Vec<Scored>> {
    binary::functions(reference)?
        .into_iter()
        .map(|name| {
            let reference_size = binary::function(reference, &name)?.bytes.len();
            Ok(Scored {
                unit: None,
                name,
                grade: Grade::Missing,
                size: None,
                reference_size,
                matching_percent: 0.0,
            })
        })
        .collect()
}

/// The totals over `scored`.
pub(crate) fn total(scored: Vec<Scored>) -> Summary {
    let count = |grade: Grade| scored.iter().filter(|item| item.grade == grade).count();
    let (exact, reloc) = (count(Grade::Exact), count(Grade::Reloc));
    Summary {
        functions: scored.len(),
        exact,
        reloc,
        different: count(Grade::Different),
        missing: count(Grade::Missing),
        matched_percent: percent(exact + reloc, scored.len()),
        symbols: scored,
    }
}

/// `part / whole` as a percentage rounded to two decimals.
pub(crate) fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 100.0;
    }
    let ratio = f64::from(u32::try_from(part).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(whole).unwrap_or(u32::MAX));
    (ratio * 10_000.0).round() / 100.0
}

/// The bytes with each of `fields` set to zero.
fn masked(function: &Function, fields: &[(usize, usize)]) -> Vec<u8> {
    let mut bytes = function.bytes.clone();
    for (at, length) in fields {
        let end = at.saturating_add(*length).min(bytes.len());
        if let Some(field) = bytes.get_mut(*at..end) {
            field.fill(0);
        }
    }
    bytes
}

fn grade(wanted: &Function, got: &Function) -> (Grade, f64) {
    if wanted == got {
        return (Grade::Exact, 100.0);
    }
    let fields: Vec<(usize, usize)> = wanted
        .relocated
        .iter()
        .chain(&got.relocated)
        .copied()
        .collect();
    let (left, right) = (masked(wanted, &fields), masked(got, &fields));
    if left == right && wanted.relocated == got.relocated {
        return (Grade::Reloc, 100.0);
    }
    let length = left.len().max(right.len());
    let agreeing = (0..length)
        .filter(|at| {
            left.get(*at)
                .is_some_and(|byte| right.get(*at) == Some(byte))
        })
        .count();
    // A function that differs never rounds up to a full match.
    (Grade::Different, percent(agreeing, length).min(99.99))
}
