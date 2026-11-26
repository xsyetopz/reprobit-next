//! Add-ons: catalog tools of kind `addon` that rewrite a step's object after
//! it compiles and propose rewrites to `rbit grind`, so the community can
//! extend `ReproBit` for one compiler, such as MSVC 4.x, without a release.
//! An add-on is a program in any language that answers two commands, run in
//! the project directory:
//!
//! - `<addon> rewrite <object> <args>…` edits the object in place, or exits
//!   non-zero with the reason on stderr.
//! - `<addon> propose <built> <reference> <symbol>` prints a JSON array of
//!   argument lists, each a rewrite that may make `symbol` in the built
//!   object match the reference.

use std::path::{Path, PathBuf};
use std::process::Output;

use anyhow::{Context, Result, ensure};

use crate::catalog::Catalog;
use crate::project::Loaded;
use crate::tools;

/// An installed add-on the project lists.
pub(crate) struct Addon {
    id: String,
    command: PathBuf,
    root: PathBuf,
    /// The install's content digest, so a changed add-on invalidates the cache.
    pub(crate) tree: String,
}

impl Addon {
    /// Opens add-on `id`, which `toolchain.tools` must list.
    pub(crate) fn open(catalog: &Catalog, loaded: &Loaded, id: &str) -> Result<Self> {
        ensure!(
            loaded.project.toolchain.tools.iter().any(|tool| tool == id),
            "add-on {id} is not in toolchain.tools"
        );
        let installed = tools::find(catalog, id)?
            .with_context(|| format!("{id} is not installed; run `rbit setup`"))?;
        ensure!(
            installed.tool.kind == "addon",
            "{id} is a {}, not an add-on",
            installed.tool.kind
        );
        let name = installed.tool.bin.contains_key(id).then_some(id);
        Ok(Self {
            id: id.to_owned(),
            command: tools::command(&installed, name)?,
            root: loaded.root.clone(),
            tree: installed.receipt.tree,
        })
    }

    /// Every add-on the project lists.
    pub(crate) fn all(catalog: &Catalog, loaded: &Loaded) -> Result<Vec<Self>> {
        loaded
            .project
            .toolchain
            .tools
            .iter()
            .filter(|id| catalog.get(id).is_ok_and(|tool| tool.kind == "addon"))
            .map(|id| Self::open(catalog, loaded, id))
            .collect()
    }

    /// The add-on's catalog id.
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    /// Rewrites `object` in place.
    pub(crate) fn rewrite(&self, object: &Path, args: &[String]) -> Result<()> {
        self.run(
            &[
                &["rewrite".as_ref(), object.as_os_str()][..],
                &args.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
            ]
            .concat(),
        )
        .map(drop)
    }

    /// The rewrites the add-on proposes for `symbol`.
    pub(crate) fn propose(
        &self,
        built: &Path,
        reference: &Path,
        symbol: &str,
    ) -> Result<Vec<Vec<String>>> {
        let output = self.run(&[
            "propose".as_ref(),
            built.as_os_str(),
            reference.as_os_str(),
            symbol.as_ref(),
        ])?;
        serde_json::from_slice(&output.stdout).with_context(|| {
            format!(
                "add-on {} proposed something other than a JSON array of argument lists",
                self.id
            )
        })
    }

    fn run(&self, args: &[&std::ffi::OsStr]) -> Result<Output> {
        let output = tools::launch(&self.command)
            .args(args)
            .current_dir(&self.root)
            .output()
            .with_context(|| format!("cannot start add-on {}", self.id))?;
        ensure!(
            output.status.success(),
            "add-on {} failed: {}",
            self.id,
            String::from_utf8_lossy(&output.stderr).trim()
        );
        Ok(output)
    }
}
