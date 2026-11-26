//! SHA-256 digests in the spellings `sha256sum` prints, so anyone can
//! recompute a pin without reprobit.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256 of `bytes`.
pub(crate) fn bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Lowercase hex SHA-256 of one file, streamed.
pub(crate) fn file(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut reader =
        fs::File::open(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Every regular file below `root`, as sorted `/`-separated relative paths.
/// Entries named in `skip` (at any depth) are left out.
pub(crate) fn files(root: &Path, skip: &[&str]) -> Result<Vec<String>> {
    let mut found = Vec::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        let directory = root.join(&relative);
        let entries = fs::read_dir(&directory)
            .with_context(|| format!("cannot list {}", directory.display()))?;
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if skip.contains(&name.as_str()) {
                continue;
            }
            let path = relative.join(&name);
            let kind = entry.file_type()?;
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() || entry.path().is_file() {
                found.push(path.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found.sort();
    Ok(found)
}

/// The SHA-256 of a tree's `sha256sum`-style listing: one `<digest>  <path>`
/// line per file in sorted order. `cd root && find . -type f | sort | …`
/// reproduces it with standard tools.
pub(crate) fn tree(root: &Path, skip: &[&str]) -> Result<String> {
    let mut listing = String::new();
    for relative in files(root, skip)? {
        listing.push_str(&file(&root.join(&relative))?);
        listing.push_str("  ");
        listing.push_str(&relative);
        listing.push('\n');
    }
    Ok(bytes(listing.as_bytes()))
}
