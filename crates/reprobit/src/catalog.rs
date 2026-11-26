//! The tool catalog: a TOML document that names every downloadable tool and
//! compiler, where to fetch it, and how to check it. reprobit ships one
//! catalog and reads more from files or URLs listed in `REPROBIT_CATALOGS`, so
//! a team can publish its own without a reprobit release.

use std::collections::BTreeMap;
use std::env;
use std::fs;

use anyhow::{Context, Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::home;

/// The catalog compiled into `rbit`.
const BUILT_IN: &str = include_str!("../../../catalog/tools.toml");

/// A whole catalog document.
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Catalog {
    /// Tools keyed by id, such as `objdiff-cli` or `msvc-4.2`.
    #[serde(default)]
    pub(crate) tools: BTreeMap<String, Tool>,
    /// Families of tools that share one fetch template, such as decomp.me's
    /// compiler images. Each member becomes the tool `<collection>/<platform>/<name>`.
    #[serde(default)]
    pub(crate) collections: BTreeMap<String, Collection>,
}

/// One downloadable tool or compiler.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Tool {
    /// `compiler`, `tool`, or `runtime` (something other tools run on, like Wine).
    #[serde(default = "default_kind")]
    pub(crate) kind: String,
    /// One line on what it is for.
    #[serde(default)]
    pub(crate) summary: String,
    /// Upstream version this entry pins.
    #[serde(default)]
    pub(crate) version: String,
    /// Upstream home page.
    #[serde(default)]
    pub(crate) homepage: String,
    /// SPDX expression, or `proprietary` for redistributed vendor compilers.
    #[serde(default)]
    pub(crate) license: String,
    /// Targets and topics, such as `ps2`, `gc`, `x86`, `diff`.
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    /// Where the tool's executables run: `host` (native for the downloaded
    /// platform), `windows` (Win32 programs: Wine, wibo, or Docker off
    /// Windows), `linux-x86_64` or `linux-i386` (Docker off Linux), or `detect` (read
    /// from the installed executables' formats).
    #[serde(default = "default_runs")]
    pub(crate) runs: String,
    /// Command names mapped to paths inside the install.
    #[serde(default)]
    pub(crate) bin: BTreeMap<String, String>,
    /// Environment for the tool's commands. `{toolchain}` and `{temp}` expand
    /// to the compiler-visible spellings of those roots.
    #[serde(default)]
    pub(crate) env: BTreeMap<String, String>,
    /// Arguments that force-include a header, with `{}` for its path, such as
    /// `["/FI{}"]` for MSVC or `["-include", "{}"]` for GCC.
    #[serde(default)]
    pub(crate) force_include: Vec<String>,
    /// Commands that must be on `PATH`, such as `java` or `node`.
    #[serde(default)]
    pub(crate) needs: Vec<String>,
    /// Fetches, applied in order into one install directory.
    #[serde(default)]
    pub(crate) get: Vec<Get>,
    /// Byte patches with a checked preimage, applied after every fetch.
    #[serde(default)]
    pub(crate) patch: Vec<Patch>,
    /// SHA-256 of files that must match after install, keyed by relative path.
    #[serde(default)]
    pub(crate) verify: BTreeMap<String, String>,
}

/// One fetch. Exactly one of `url`, `git`, `oci`, `pypi`, or `npm` is set.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Get {
    /// Host platforms this fetch applies to, such as `macos-aarch64`; empty means all.
    #[serde(default)]
    pub(crate) host: Vec<String>,
    /// A zip, a plain, gzip, or xz tar, or a single file, recognized by content.
    pub(crate) url: Option<String>,
    /// SHA-256 of the downloaded `url`.
    pub(crate) sha256: Option<String>,
    /// File name for a single-file `url`; defaults to the URL's last segment.
    #[serde(rename = "as")]
    pub(crate) save_as: Option<String>,
    /// A git repository, fetched shallowly at `rev`.
    pub(crate) git: Option<String>,
    /// The full commit for `git`.
    pub(crate) rev: Option<String>,
    /// An OCI image such as `ghcr.io/owner/name`; every layer is unpacked.
    pub(crate) oci: Option<String>,
    /// Image tag, or the `sha256:` manifest digest to pin.
    #[serde(default)]
    pub(crate) reference: Option<String>,
    /// A Python requirement installed into a private virtual environment.
    pub(crate) pypi: Option<String>,
    /// An npm package installed into a private prefix.
    pub(crate) npm: Option<String>,
    /// Paths to copy from the fetched tree; `from = "to"`. Empty copies everything.
    #[serde(default)]
    pub(crate) pick: BTreeMap<String, String>,
    /// Leading directory to drop from archive and image entries.
    #[serde(default)]
    pub(crate) strip: String,
}

/// A byte patch whose preimage must match before it is applied.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Patch {
    /// File relative to the install root.
    pub(crate) file: String,
    /// Byte offset.
    pub(crate) offset: u64,
    /// Expected bytes, hex.
    pub(crate) from: String,
    /// Replacement bytes, hex, the same length as `from`.
    pub(crate) to: String,
}

/// A family of tools built from one template.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Collection {
    /// One line on where the members come from.
    #[serde(default)]
    pub(crate) summary: String,
    /// Upstream home page.
    #[serde(default)]
    pub(crate) homepage: String,
    /// License note shared by the members.
    #[serde(default)]
    pub(crate) license: String,
    /// OCI image template with `{platform}` and `{name}`.
    pub(crate) oci: String,
    /// Leading directory inside each image, with the same placeholders.
    #[serde(default)]
    pub(crate) strip: String,
    /// Where members run, keyed by platform; the rest use `default-runs`.
    #[serde(default)]
    pub(crate) runs: BTreeMap<String, String>,
    /// Where members run when `runs` does not name their platform. `detect`
    /// reads the installed executables' formats.
    #[serde(default = "default_runs")]
    pub(crate) default_runs: String,
    /// Members published as one image per host OS, keyed by `platform/name`:
    /// the OS names (`linux`, `darwin`) appended to the image path. macOS
    /// hosts take `darwin` when it is listed; every other host takes `linux`.
    #[serde(default)]
    pub(crate) split: BTreeMap<String, Vec<String>>,
    /// Force-include arguments keyed by platform, as in [`Tool::force_include`].
    #[serde(default)]
    pub(crate) force_include: BTreeMap<String, Vec<String>>,
    /// Member names keyed by target platform, such as `ps2 = ["ee-gcc2.95.3-136"]`.
    pub(crate) platforms: BTreeMap<String, Vec<String>>,
}

fn default_kind() -> String {
    "tool".into()
}

fn default_runs() -> String {
    "host".into()
}

impl Catalog {
    /// The built-in catalog merged with each source in `REPROBIT_CATALOGS`
    /// (paths or URLs separated by the platform's path-list separator).
    /// A later catalog replaces an earlier entry with the same id.
    pub(crate) fn load() -> Result<Self> {
        let mut catalog = Self::parse(BUILT_IN, "the built-in catalog")?;
        if let Some(list) = env::var_os("REPROBIT_CATALOGS") {
            for source in env::split_paths(&list) {
                let source = source.to_string_lossy().into_owned();
                if source.is_empty() {
                    continue;
                }
                let text = if source.starts_with("https://") || source.starts_with("http://") {
                    crate::fetch::text(&source, &[])?
                } else {
                    fs::read_to_string(&source)
                        .with_context(|| format!("cannot read catalog {source}"))?
                };
                let extra = Self::parse(&text, &source)?;
                catalog.tools.extend(extra.tools);
                catalog.collections.extend(extra.collections);
            }
        }
        Ok(catalog)
    }

    fn parse(text: &str, origin: &str) -> Result<Self> {
        toml::from_str(text).with_context(|| format!("{origin} is not a valid catalog"))
    }

    /// Every tool id, collection members included, in sorted order.
    pub(crate) fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.tools.keys().cloned().collect();
        for (collection, family) in &self.collections {
            for (platform, names) in &family.platforms {
                ids.extend(
                    names
                        .iter()
                        .map(|name| format!("{collection}/{platform}/{name}")),
                );
            }
        }
        ids.sort();
        ids
    }

    /// The tool named `id`, expanding a collection member on demand.
    ///
    /// # Errors
    ///
    /// Fails when no tool or collection member has that id.
    pub(crate) fn get(&self, id: &str) -> Result<Tool> {
        if let Some(tool) = self.tools.get(id) {
            return Ok(tool.clone());
        }
        let mut parts = id.splitn(3, '/');
        if let (Some(collection), Some(platform), Some(name)) =
            (parts.next(), parts.next(), parts.next())
            && let Some(family) = self.collections.get(collection)
            && family
                .platforms
                .get(platform)
                .is_some_and(|names| names.iter().any(|known| known == name))
        {
            let fill = |template: &str| {
                template
                    .replace("{platform}", platform)
                    .replace("{name}", name)
            };
            let mut image = fill(&family.oci).to_lowercase().replace('+', "plus");
            let strip = fill(&family.strip);
            if let Some(oses) = family.split.get(&format!("{platform}/{name}")) {
                let os = if cfg!(target_os = "macos") && oses.iter().any(|os| os == "darwin") {
                    "darwin"
                } else {
                    "linux"
                };
                image = format!("{image}/{os}");
            }
            return Ok(Tool {
                kind: "compiler".into(),
                summary: format!("{name} for {platform}, from {collection}"),
                version: "latest".into(),
                homepage: family.homepage.clone(),
                license: family.license.clone(),
                tags: vec![platform.to_owned()],
                runs: family
                    .runs
                    .get(platform)
                    .unwrap_or(&family.default_runs)
                    .clone(),
                force_include: family
                    .force_include
                    .get(platform)
                    .cloned()
                    .unwrap_or_default(),
                get: vec![Get {
                    oci: Some(image),
                    strip,
                    ..Get::default()
                }],
                ..Tool::default()
            });
        }
        let close: Vec<String> = self
            .ids()
            .into_iter()
            .filter(|known| known.contains(id))
            .take(5)
            .collect();
        if close.is_empty() {
            bail!("no tool named {id}; `rbit tool list` shows the catalog");
        }
        bail!("no tool named {id}; did you mean {}?", close.join(", "))
    }
}

impl Tool {
    /// The fetches that apply on this host.
    pub(crate) fn fetches(&self) -> Vec<&Get> {
        let host = home::host();
        self.get
            .iter()
            .filter(|get| get.host.is_empty() || get.host.contains(&host))
            .collect()
    }
}
