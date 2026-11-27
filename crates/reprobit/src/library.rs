//! `rbit locate`: finds where each function of a library's objects sits in a linked
//! binary. Library code is verified against its original objects rather than decompiled,
//! so a function counts only when its bytes appear at one place, with every field the
//! linker patches left out of the comparison.

use anyhow::{Context, Result};
use object::read::archive::ArchiveFile;
use object::{Object, ObjectSection, SectionKind};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::binary;

/// Where one library function was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Status {
    /// At exactly one place.
    Found,
    /// At more than one place, so the bytes do not say which copy is the library's.
    Ambiguous,
    /// Nowhere.
    Absent,
}

/// One library function's result.
#[derive(Debug, Serialize)]
pub(crate) struct Placed {
    pub(crate) name: String,
    pub(crate) status: Status,
    pub(crate) size: usize,
    /// The binary's address of the function, when `found`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) address: Option<u64>,
    /// How many places hold the bytes, when `ambiguous`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) places: Option<usize>,
    /// `member:function` of each other function found at the same place.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) shared: Vec<String>,
}

/// One object's results.
#[derive(Debug, Serialize)]
pub(crate) struct Member {
    /// The archive member's name, or the object's file name.
    #[serde(rename = "member")]
    pub(crate) name: String,
    /// Every function the object defines is `found`.
    pub(crate) linked: bool,
    pub(crate) functions: Vec<Placed>,
    /// Absent functions left out of `functions`, without `--all`.
    #[serde(skip_serializing_if = "is_zero")]
    pub(crate) absent: usize,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
const fn is_zero(count: &usize) -> bool {
    *count == 0
}

/// Every object's results and the totals.
#[derive(Debug, Serialize)]
pub(crate) struct Summary {
    pub(crate) members: Vec<Member>,
    pub(crate) linked: usize,
    pub(crate) found: usize,
    pub(crate) found_bytes: usize,
    /// Members left out of `members` because none of their functions was placed.
    pub(crate) unplaced: usize,
}

/// The objects in `bytes`: each member of an `ar` archive, or the object itself.
pub(crate) fn objects(name: &str, bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>> {
    if !bytes.starts_with(b"!<arch>\n") {
        return Ok(vec![(name.to_owned(), bytes.to_vec())]);
    }
    let archive = ArchiveFile::parse(bytes).with_context(|| format!("cannot read {name}"))?;
    archive
        .members()
        .map(|member| {
            let member = member?;
            Ok((
                String::from_utf8_lossy(member.name()).into_owned(),
                member.data(bytes)?.to_vec(),
            ))
        })
        .collect()
}

/// Finds each function of `objects` in the code sections of `binary`. A member that is
/// not an object, such as an import stub, or defines no function, is left out. Unless
/// `all`, so is a member with nothing `found` or `ambiguous`: a vendor archive holds
/// hundreds of objects a binary never links, and listing them buries the ones it does.
pub(crate) fn locate(binary: &[u8], objects: &[(String, Vec<u8>)], all: bool) -> Result<Summary> {
    let file = object::File::parse(binary).context("cannot read the binary")?;
    let code = file
        .sections()
        .filter(|section| section.kind() == SectionKind::Text)
        .map(|section| Ok((section.address(), section.data()?)))
        .collect::<Result<Vec<_>>>()?;
    let mut members = Vec::new();
    for (member, bytes) in objects {
        let Ok(names) = binary::functions(bytes) else {
            continue;
        };
        let mut functions = Vec::new();
        for name in names {
            let Ok(function) = binary::function(bytes, &name) else {
                continue;
            };
            if function.bytes.is_empty() {
                continue;
            }
            let mut kept = vec![true; function.bytes.len()];
            for &(offset, width) in &function.relocated {
                kept.iter_mut()
                    .skip(offset)
                    .take(width)
                    .for_each(|keep| *keep = false);
            }
            let (count, first) = places(&code, &function.bytes, &kept);
            let status = match count {
                0 => Status::Absent,
                1 => Status::Found,
                _ => Status::Ambiguous,
            };
            functions.push(Placed {
                name,
                status,
                size: function.bytes.len(),
                address: first.filter(|_| count == 1),
                places: (count > 1).then_some(count),
                shared: Vec::new(),
            });
        }
        if functions.is_empty() {
            continue;
        }
        members.push(Member {
            name: member.clone(),
            linked: false,
            functions,
            absent: 0,
        });
    }
    share(&mut members);
    for member in &mut members {
        member.linked = member
            .functions
            .iter()
            .all(|item| item.status == Status::Found);
    }
    let before = members.len();
    if !all {
        members.retain(|member| {
            member
                .functions
                .iter()
                .any(|item| item.status != Status::Absent)
        });
        for member in &mut members {
            let before = member.functions.len();
            member
                .functions
                .retain(|item| item.status != Status::Absent);
            member.absent = before - member.functions.len();
        }
    }
    let unplaced = before - members.len();
    let found = members
        .iter()
        .flat_map(|member| &member.functions)
        .filter(|item| item.status == Status::Found);
    Ok(Summary {
        linked: members.iter().filter(|member| member.linked).count(),
        found: found.clone().count(),
        found_bytes: found.map(|item| item.size).sum(),
        unplaced,
        members,
    })
}

/// Settles each place where functions of several members were found. Short functions
/// and variant builds of one source (`printf` and `wprintf` once relocations are masked)
/// share bytes, so the place stays `found` only for a claimant whose member has every
/// other function found, when exactly one has; every other claimant is `ambiguous`.
fn share(members: &mut [Member]) {
    let complete: Vec<bool> = members
        .iter()
        .map(|member| {
            member
                .functions
                .iter()
                .all(|item| item.status == Status::Found)
        })
        .collect();
    let mut claims: BTreeMap<u64, Vec<(usize, usize)>> = BTreeMap::new();
    for (member, entry) in members.iter().enumerate() {
        for (function, item) in entry.functions.iter().enumerate() {
            if let (Status::Found, Some(address)) = (item.status, item.address) {
                claims.entry(address).or_default().push((member, function));
            }
        }
    }
    for claimants in claims.into_values().filter(|claimants| claimants.len() > 1) {
        let whole: Vec<(usize, usize)> = claimants
            .iter()
            .copied()
            .filter(|&(member, _)| complete[member])
            .collect();
        let keep = match whole.as_slice() {
            [only] => Some(*only),
            _ => None,
        };
        let names: Vec<String> = claimants
            .iter()
            .map(|&(member, function)| {
                format!(
                    "{}:{}",
                    members[member].name, members[member].functions[function].name
                )
            })
            .collect();
        for (index, &(member, function)) in claimants.iter().enumerate() {
            if Some((member, function)) == keep {
                continue;
            }
            let item = &mut members[member].functions[function];
            item.status = Status::Ambiguous;
            item.address = None;
            item.places = Some(1);
            item.shared = names
                .iter()
                .enumerate()
                .filter(|&(other, _)| other != index)
                .map(|(_, name)| name.clone())
                .collect();
        }
    }
}

/// How many places in `code` equal `bytes` at every position `kept` marks, and the
/// address of the first. The longest kept run is compared first, because it rejects most
/// places in one slice comparison.
fn places(code: &[(u64, &[u8])], bytes: &[u8], kept: &[bool]) -> (usize, Option<u64>) {
    let (mut anchor, mut run) = ((0, 0), 0);
    for (index, keep) in kept.iter().enumerate() {
        run = if *keep { run + 1 } else { 0 };
        if run > anchor.1 {
            anchor = (index + 1 - run, run);
        }
    }
    let (at, length) = anchor;
    let wanted = &bytes[at..at + length];
    let (mut count, mut first) = (0, None);
    for (address, data) in code {
        let Some(last) = data.len().checked_sub(bytes.len()) else {
            continue;
        };
        for start in 0..=last {
            if data[start + at..start + at + length] != *wanted {
                continue;
            }
            let equal = bytes
                .iter()
                .zip(kept)
                .zip(&data[start..])
                .all(|((want, keep), have)| !keep || want == have);
            if equal {
                count += 1;
                first = first.or(Some(address + start as u64));
            }
        }
    }
    (count, first)
}
