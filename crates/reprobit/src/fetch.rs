//! Downloads and unpacking. Every fetch is checked against a digest before
//! it reaches an install directory: a `sha256` for URLs, a commit for git,
//! and content-addressed layers for OCI images.

use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

use crate::{digest, home, ui};

/// A `curl` request for `url`. Every supported host ships curl, and it
/// brings the system's certificates and proxy settings with it.
fn curl(url: &str, headers: &[(&str, String)]) -> Command {
    let mut command = Command::new("curl");
    command
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--retry",
            "3",
        ])
        .stdin(std::process::Stdio::null());
    for (key, value) in headers {
        command.arg("--header").arg(format!("{key}: {value}"));
    }
    command.arg(url);
    command
}

/// The body of `url` as text.
pub(crate) fn text(url: &str, headers: &[(&str, String)]) -> Result<String> {
    let output = curl(url, headers)
        .output()
        .context("downloading needs curl on PATH")?;
    ensure!(
        output.status.success(),
        "cannot fetch {url}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8(output.stdout)?)
}

/// Downloads `url` into the download cache and returns its path, checked
/// against `sha256` when given. A cached copy with the right digest is reused.
pub(crate) fn download(
    url: &str,
    sha256: Option<&str>,
    headers: &[(&str, String)],
) -> Result<PathBuf> {
    // A local Windows path separates its parts with `\`.
    let name = url
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("download")
        .split('?')
        .next()
        .unwrap_or("download")
        // Registry blobs end in `sha256:<hex>`; Windows forbids `:` in file names.
        .replace(':', "-");
    let directory = home::downloads().join(sha256.unwrap_or("unpinned"));
    let path = directory.join(&name);
    if let Some(expected) = sha256
        && path.is_file()
        && digest::file(&path)? == expected
    {
        return Ok(path);
    }
    fs::create_dir_all(&directory)?;
    let partial = directory.join(format!(".{name}.partial"));
    // Local paths and file:// URLs serve offline mirrors and air-gapped CI.
    if let Some(local) = url
        .strip_prefix("file://")
        .or_else(|| (!url.contains("://")).then_some(url))
    {
        fs::copy(local, &partial).with_context(|| format!("cannot read {local}"))?;
        return finish(url, &partial, &path, sha256);
    }
    ui::note(&format!("Downloading {url}"));
    let status = curl(url, headers)
        .arg("--output")
        .arg(&partial)
        .stderr(ui::child_output())
        .status()
        .context("downloading needs curl on PATH")?;
    ensure!(status.success(), "cannot fetch {url}");
    finish(url, &partial, &path, sha256)
}

/// Moves a finished download into place once its digest matches.
fn finish(url: &str, partial: &Path, path: &Path, sha256: Option<&str>) -> Result<PathBuf> {
    let actual = digest::file(partial)?;
    if let Some(expected) = sha256
        && actual != expected
    {
        fs::remove_file(partial)?;
        bail!("{url} has SHA-256 {actual}, but the catalog pins {expected}");
    }
    fs::rename(partial, path)?;
    Ok(path.to_path_buf())
}

/// Unpacks an archive into `into`, dropping the leading `strip` directory.
/// A path that is not an archive is copied as the single file `save_as`.
pub(crate) fn unpack(archive: &Path, into: &Path, strip: &str, save_as: &str) -> Result<()> {
    // Formats are recognized by content: registry blobs have no extension.
    let mut magic = Vec::new();
    fs::File::open(archive)?.take(262).read_to_end(&mut magic)?;
    let file = fs::File::open(archive)?;
    if magic.starts_with(b"PK\x03\x04") {
        let mut zip = zip::ZipArchive::new(file)?;
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index)?;
            let Some(relative) = entry
                .enclosed_name()
                .and_then(|path| stripped(&path, strip))
            else {
                continue;
            };
            let target = into.join(relative);
            if entry.is_dir() {
                fs::create_dir_all(&target)?;
                continue;
            }
            let mode = entry.unix_mode().unwrap_or(0o644);
            write_entry(&target, &mut entry, mode)?;
        }
        Ok(())
    } else if magic.starts_with(&[0x1f, 0x8b]) {
        untar(
            flate2::read::GzDecoder::new(io::BufReader::new(file)),
            into,
            strip,
        )
    } else if magic.starts_with(b"\xfd7zXZ\0") {
        let mut decompressed = Vec::new();
        lzma_rs::xz_decompress(&mut io::BufReader::new(file), &mut decompressed)
            .map_err(|error| anyhow::anyhow!("cannot decompress {}: {error}", archive.display()))?;
        untar(decompressed.as_slice(), into, strip)
    } else if magic.get(257..262) == Some(b"ustar") {
        untar(io::BufReader::new(file), into, strip)
    } else {
        let target = into.join(save_as);
        write_entry(&target, &mut fs::File::open(archive)?, 0o755)
    }
}

fn untar(reader: impl Read, into: &Path, strip: &str) -> Result<()> {
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let Some(relative) = stripped(&path, strip) else {
            continue;
        };
        let target = into.join(relative);
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            fs::create_dir_all(&target)?;
        } else if kind.is_file() {
            let mode = entry.header().mode().unwrap_or(0o644);
            write_entry(&target, &mut entry, mode)?;
        } else if kind.is_symlink() || kind.is_hard_link() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            entry.unpack(&target)?;
        }
    }
    Ok(())
}

/// The entry path below `strip`, or `None` when it is outside it or escapes
/// the install root. Only normal components survive.
fn stripped(path: &Path, strip: &str) -> Option<PathBuf> {
    let parts: Vec<_> = path
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name.to_os_string()),
            _ => None,
        })
        .collect();
    let prefix: Vec<&str> = strip.split('/').filter(|part| !part.is_empty()).collect();
    let inside = parts.len() > prefix.len()
        && parts
            .iter()
            .zip(&prefix)
            .all(|(part, want)| part.to_string_lossy() == *want);
    inside.then(|| parts[prefix.len()..].iter().collect())
}

fn write_entry(target: &Path, reader: &mut impl Read, mode: u32) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    io::copy(reader, &mut fs::File::create(target)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            target,
            fs::Permissions::from_mode(if mode & 0o111 == 0 { 0o644 } else { 0o755 }),
        )?;
    }
    #[cfg(not(unix))]
    let _ = mode;
    Ok(())
}

/// Shallow-fetches one commit of `url` into `into` and checks that the
/// checkout really is that commit.
pub(crate) fn git(url: &str, rev: &str, paths: &[&str], into: &Path) -> Result<()> {
    fs::create_dir_all(into)?;
    let run = |args: &[&str]| -> Result<()> {
        let status = Command::new("git")
            .arg("-C")
            .arg(into)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(ui::child_output())
            .status()?;
        ensure!(status.success(), "git {} failed for {url}", args.join(" "));
        Ok(())
    };
    ui::note(&format!("Fetching {url} at {}", &rev[..rev.len().min(12)]));
    run(&["init", "--quiet"])?;
    run(&["config", "core.autocrlf", "false"])?;
    run(&["remote", "add", "origin", url])?;
    if !paths.is_empty() {
        let mut args = vec!["sparse-checkout", "set", "--no-cone"];
        args.extend(paths);
        run(&args)?;
    }
    run(&["fetch", "--quiet", "--depth=1", "origin", rev])?;
    run(&[
        "-c",
        "advice.detachedHead=false",
        "checkout",
        "--quiet",
        "--detach",
        "FETCH_HEAD",
    ])?;
    let head = Command::new("git")
        .arg("-C")
        .arg(into)
        .args(["rev-parse", "HEAD"])
        .output()?;
    let head = String::from_utf8_lossy(&head.stdout).trim().to_owned();
    ensure!(
        head == rev,
        "{url} checked out {head}, not the pinned {rev}"
    );
    fs::remove_dir_all(into.join(".git"))?;
    Ok(())
}

/// Pulls an OCI image anonymously and unpacks its layers into `into`.
/// Returns the manifest digest, which pins the exact image for the lock.
pub(crate) fn oci(image: &str, reference: &str, strip: &str, into: &Path) -> Result<String> {
    let (registry, repository) = image
        .split_once('/')
        .context("an OCI image needs a registry host")?;
    let token_url = format!("https://{registry}/token?scope=repository:{repository}:pull");
    let token: Value = serde_json::from_str(&text(&token_url, &[])?)?;
    let auth = format!("Bearer {}", token["token"].as_str().unwrap_or_default());
    let accept = "application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, \
                  application/vnd.docker.distribution.manifest.list.v2+json, \
                  application/vnd.docker.distribution.manifest.v2+json";
    let manifest = |reference: &str| -> Result<(Value, String)> {
        let url = format!("https://{registry}/v2/{repository}/manifests/{reference}");
        let body = text(
            &url,
            &[
                ("Authorization", auth.clone()),
                ("Accept", accept.to_owned()),
            ],
        )?;
        Ok((
            serde_json::from_str(&body)?,
            format!("sha256:{}", digest::bytes(body.as_bytes())),
        ))
    };
    let (mut document, mut pinned) = manifest(reference)?;
    if let Some(list) = document["manifests"].as_array() {
        let chosen = list
            .iter()
            .find(|item| item["platform"]["architecture"] == "amd64")
            .or_else(|| list.first())
            .and_then(|item| item["digest"].as_str())
            .context("the image index lists no manifests")?
            .to_owned();
        (document, pinned) = manifest(&chosen)?;
    }
    if reference.starts_with("sha256:") {
        ensure!(
            pinned == reference,
            "{image} manifest is {pinned}, not the pinned {reference}"
        );
    }
    for layer in document["layers"]
        .as_array()
        .context("the manifest lists no layers")?
    {
        let digest = layer["digest"].as_str().context("a layer has no digest")?;
        let url = format!("https://{registry}/v2/{repository}/blobs/{digest}");
        let expected = digest.trim_start_matches("sha256:");
        let blob = download(&url, Some(expected), &[("Authorization", auth.clone())])?;
        unpack(&blob, into, strip, "")?;
    }
    Ok(pinned)
}

/// Installs a Python requirement into a private virtual environment at `into`.
pub(crate) fn pypi(requirement: &str, into: &Path) -> Result<()> {
    let python = ["python3", "python"]
        .into_iter()
        .find(|name| {
            Command::new(name)
                .arg("--version")
                .output()
                .is_ok_and(|out| out.status.success())
        })
        .context("installing a Python tool needs python3 on PATH")?;
    ui::note(&format!("Installing {requirement} with pip"));
    let status = Command::new(python)
        .arg("-m")
        .arg("venv")
        .arg(into)
        .stdout(ui::child_output())
        .status()?;
    ensure!(
        status.success(),
        "cannot create a virtual environment at {}",
        into.display()
    );
    let pip = into.join(if cfg!(windows) {
        "Scripts/pip.exe"
    } else {
        "bin/pip"
    });
    let status = Command::new(pip)
        .args([
            "install",
            "--quiet",
            "--disable-pip-version-check",
            requirement,
        ])
        .stdout(ui::child_output())
        .status()?;
    ensure!(status.success(), "pip could not install {requirement}");
    Ok(())
}

/// Installs an npm package into a private prefix at `into`.
pub(crate) fn npm(package: &str, into: &Path) -> Result<()> {
    ui::note(&format!("Installing {package} with npm"));
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let status = Command::new(npm)
        .args(["install", "--silent", "--no-fund", "--no-audit", "--prefix"])
        .arg(into)
        .arg(package)
        .stdout(ui::child_output())
        .status()
        .context("installing a Node tool needs npm on PATH")?;
    ensure!(status.success(), "npm could not install {package}");
    Ok(())
}
