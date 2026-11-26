//! Machine-local directories. Everything reprobit downloads lives under one
//! root so `rbit tool remove --all` (or deleting that folder) undoes it.

use std::env;
use std::path::PathBuf;

/// The root for installed tools, caches, and settings.
///
/// `REPROBIT_HOME` wins so CI, tests, and portable installs can relocate it.
/// Otherwise the platform's per-user data directory is used, matching where
/// earlier releases kept their toolchains.
pub(crate) fn root() -> PathBuf {
    if let Some(path) = env::var_os("REPROBIT_HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    if cfg!(windows) {
        env::var_os("LOCALAPPDATA")
            .map_or_else(|| home.join("AppData").join("Local"), PathBuf::from)
            .join("ReproBit")
    } else if cfg!(target_os = "macos") {
        home.join("Library/Application Support/ReproBit")
    } else {
        env::var_os("XDG_DATA_HOME")
            .filter(|value| !value.is_empty())
            .map_or_else(|| home.join(".local/share"), PathBuf::from)
            .join("reprobit")
    }
}

/// Where each catalog tool is unpacked: `<root>/tools/<id>/<version>`.
pub(crate) fn tools() -> PathBuf {
    root().join("tools")
}

/// Downloaded archives kept for re-installs; safe to delete at any time.
pub(crate) fn downloads() -> PathBuf {
    root().join("downloads")
}

/// The host platform in catalog spelling, such as `macos-aarch64`.
pub(crate) fn host() -> String {
    format!("{}-{}", env::consts::OS, env::consts::ARCH)
}
