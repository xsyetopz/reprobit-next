//! `reprobit.toml`: one file that says what a project builds, with which
//! tool, through which steps, and what each output must equal.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The project file name.
pub(crate) const FILE: &str = "reprobit.toml";
/// The lock that pins each tool's installed content.
pub(crate) const LOCK: &str = "reprobit.lock";

/// A whole project file.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Project {
    /// A short name for reports.
    pub(crate) name: String,
    /// Local, ignored working state: builds, caches, and reports.
    #[serde(default = "default_state")]
    pub(crate) state: String,
    /// The compiler and the tools its steps call.
    pub(crate) toolchain: Toolchain,
    /// The paths the compiler sees. Fixed spellings keep paths that a
    /// compiler writes into its output identical on every machine. Without
    /// this table, Win32 compilers under Wine see `R:\source`, `R:\build`,
    /// `R:\toolchain`, and `R:\Users\reprobit\AppData\Local\Temp`; others
    /// see `/source`, `/build`, `/toolchain`, and `/tmp`.
    #[serde(default)]
    pub(crate) paths: Option<Paths>,
    /// File extensions whose contents every compile step depends on, such as
    /// headers no step lists as an input.
    #[serde(default = "default_headers")]
    pub(crate) headers: Vec<String>,
    /// Binaries to reproduce.
    #[serde(default, rename = "target")]
    pub(crate) targets: Vec<Target>,
    /// Producer steps in run order.
    #[serde(default, rename = "step")]
    pub(crate) steps: Vec<Step>,
}

/// The tools a project builds with.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Toolchain {
    /// Catalog id of the compiler, such as `msvc-4.2` or `decompme/ps2/ee-gcc2.95.3-136`.
    pub(crate) compiler: String,
    /// Other catalog tools steps may call, by id.
    #[serde(default)]
    pub(crate) tools: Vec<String>,
    /// `auto`, `native`, `wine`, `wibo`, or `docker`.
    #[serde(default = "default_runner")]
    pub(crate) runner: String,
    /// Overrides where the compiler runs (`host`, `windows`, `linux-x86_64`, `linux-i386`)
    /// when the catalog's platform-wide guess is wrong for this compiler.
    #[serde(default)]
    pub(crate) runs: Option<String>,
}

/// Compiler-visible roots.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Paths {
    /// Where the project's sources appear, such as `R:\source` or `/src`.
    pub(crate) source: String,
    /// Where outputs appear.
    pub(crate) build: String,
    /// Where the compiler's own files appear.
    pub(crate) toolchain: String,
    /// The temporary directory the compiler is given.
    pub(crate) temp: String,
}

impl Project {
    /// The declared paths, or the defaults for a compiler that sees DOS
    /// paths (`dos`) or POSIX ones.
    pub(crate) fn paths(&self, dos: bool) -> Paths {
        let (source, build, toolchain, temp) = if dos {
            (
                r"R:\source",
                r"R:\build",
                r"R:\toolchain",
                r"R:\Users\reprobit\AppData\Local\Temp",
            )
        } else {
            ("/source", "/build", "/toolchain", "/tmp")
        };
        self.paths.clone().unwrap_or_else(|| Paths {
            source: source.into(),
            build: build.into(),
            toolchain: toolchain.into(),
            temp: temp.into(),
        })
    }
}

/// One binary to reproduce.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Target {
    /// Stable id, such as `program`.
    pub(crate) id: String,
    /// The rebuilt file, relative to the build root.
    pub(crate) artifact: String,
    /// The original file, relative to the project. Project-owned and never
    /// redistributed; it is read only by verification.
    pub(crate) reference: String,
    /// SHA-256 the reference must have after `normalize`, so a wrong
    /// original is caught before any comparison.
    #[serde(default)]
    pub(crate) sha256: Option<String>,
    /// Metadata normalizations applied to both sides before comparing:
    /// `pe-timestamps`, `pe-checksum`, or `build-id`. Each clears only fields
    /// a linker fills from the clock or a random source.
    #[serde(default)]
    pub(crate) normalize: Vec<String>,
}

/// One producer invocation.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Step {
    /// Stable id, such as `compile.transform`.
    pub(crate) id: String,
    /// A command of the compiler or of a listed tool: `cl`, `objdiff-cli:objdiff-cli`,
    /// or a path inside the compiler such as `Bin/CL.EXE`.
    pub(crate) program: String,
    /// Arguments. `${SOURCE}`, `${BUILD}`, `${TOOLCHAIN}`, and `${TEMP}`
    /// expand to the compiler-visible roots.
    #[serde(default)]
    pub(crate) args: Vec<String>,
    /// Files read, as `source/…` or `build/…` (an earlier step's output).
    #[serde(default)]
    pub(crate) inputs: Vec<String>,
    /// Files written, relative to the build root.
    #[serde(default)]
    pub(crate) outputs: Vec<String>,
    /// Extra environment, with the same placeholders as `args`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) env: BTreeMap<String, String>,
    /// Declaration-only text force-included before the source: the
    /// reviewed compiler-state adjustment `rbit grind` finds.
    #[serde(default)]
    pub(crate) prelude: Option<Prelude>,
}

/// A declaration-only prelude. Every family emits declarations that produce
/// no code, data, or linker directives; they only steer compiler state.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, tag = "family", rename_all = "kebab-case")]
pub(crate) enum Prelude {
    /// `classes` classes sharing `functions` member declarations.
    Shape { classes: u32, functions: u32 },
    /// A grid of classes with `functions` unused inline members each.
    Pad { classes: u32, functions: u32 },
    /// `count` forward class declarations.
    Forward { count: u32 },
    /// `count` extern object declarations.
    Extern { count: u32 },
}

fn default_state() -> String {
    ".reprobit".into()
}

fn default_runner() -> String {
    "auto".into()
}

fn default_headers() -> Vec<String> {
    ["h", "hh", "hpp", "hxx", "inc", "inl"]
        .map(String::from)
        .to_vec()
}

/// A loaded project and the directory it lives in.
#[derive(Debug, Clone)]
pub(crate) struct Loaded {
    pub(crate) root: PathBuf,
    pub(crate) project: Project,
}

impl Loaded {
    /// Reads and checks `<dir>/reprobit.toml`.
    pub(crate) fn open(dir: &Path) -> Result<Self> {
        let path = dir.join(FILE);
        let text = fs::read_to_string(&path).with_context(|| {
            format!(
                "no {FILE} in {}; `rbit init` starts a project",
                dir.display()
            )
        })?;
        let project: Project =
            toml::from_str(&text).with_context(|| format!("{} is invalid", path.display()))?;
        let mut produced: Vec<&str> = Vec::new();
        for step in &project.steps {
            for input in &step.inputs {
                if let Some(built) = input.strip_prefix("build/") {
                    ensure!(
                        produced.contains(&built),
                        "step {} reads {input}, which no earlier step writes",
                        step.id
                    );
                } else {
                    ensure!(
                        input.starts_with("source/"),
                        "step {} input {input} must start with source/ or build/",
                        step.id
                    );
                }
            }
            produced.extend(step.outputs.iter().map(String::as_str));
        }
        for target in &project.targets {
            ensure!(
                produced.contains(&target.artifact.as_str()),
                "target {} artifact {} is not a step output",
                target.id,
                target.artifact
            );
        }
        Ok(Self {
            root: dir.canonicalize()?,
            project,
        })
    }

    /// The local state directory.
    pub(crate) fn state(&self) -> PathBuf {
        self.root.join(&self.project.state)
    }

    /// The step with `id`.
    pub(crate) fn step(&self, id: &str) -> Result<&Step> {
        match self.project.steps.iter().find(|step| step.id == id) {
            Some(step) => Ok(step),
            None => bail!(
                "no step {id}; steps are: {}",
                self.project
                    .steps
                    .iter()
                    .map(|s| s.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    /// Rewrites one step's prelude in `reprobit.toml`, or removes it when
    /// `None`, keeping the rest of the file, comments included, as it was.
    pub(crate) fn set_prelude(&self, id: &str, prelude: Option<&Prelude>) -> Result<()> {
        let path = self.root.join(FILE);
        let mut document: toml_edit::DocumentMut = fs::read_to_string(&path)?.parse()?;
        let steps = document["step"]
            .as_array_of_tables_mut()
            .context("the project has no steps")?;
        let step = steps
            .iter_mut()
            .find(|step| step["id"].as_str() == Some(id))
            .with_context(|| format!("no step {id}"))?;
        match prelude {
            None => {
                step.remove("prelude");
            }
            Some(prelude) => {
                let value: toml_edit::DocumentMut = toml::to_string(&Wrapper {
                    prelude: prelude.clone(),
                })?
                .parse()?;
                let mut inline = value["prelude"]
                    .as_table()
                    .context("prelude table")?
                    .clone()
                    .into_inline_table();
                inline.fmt();
                step["prelude"] = toml_edit::value(inline);
            }
        }
        fs::write(path, document.to_string())?;
        Ok(())
    }
}

#[derive(Serialize)]
struct Wrapper {
    prelude: Prelude,
}

impl Prelude {
    /// The declarations this prelude stands for.
    pub(crate) fn render(&self) -> String {
        match *self {
            Self::Shape { classes, functions } => shape(classes, functions),
            Self::Pad { classes, functions } => (0..classes)
                .map(|class| {
                    let members = (0..functions)
                        .map(|function| {
                            format!("\tinline void FunctionPad{class:02}x{function:02}() {{}}\n")
                        })
                        .collect::<Vec<_>>();
                    format!("class ClassPad{class:02} {{\n{}}};\n", members.concat())
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Self::Forward { count } => (0..count)
                .map(|index| format!("class Fwd{index:03};\n"))
                .collect::<Vec<_>>()
                .concat(),
            Self::Extern { count } => (0..count)
                .map(|index| format!("extern int g_p{index:03};\n"))
                .collect::<Vec<_>>()
                .concat(),
        }
    }
}

/// Declares `functions` member functions spread over `classes` classes, in
/// the naming the original MSVC campaigns used so earlier findings carry over.
fn shape(classes: u32, functions: u32) -> String {
    let classes = classes.max(1);
    let mut counts = vec![1u32; classes as usize];
    for index in 0..functions.saturating_sub(classes) {
        counts[(index % classes) as usize] += 1;
    }
    let mut out = vec![format!(
        "// Generated declaration-only entropy shape. Emits no code or data.\n// Shape: classes={classes} functions={functions}\n\n"
    )];
    let mut number = 0u32;
    for (class, count) in counts.iter().enumerate() {
        let letter = char::from(b'A' + u8::try_from(class % 26).unwrap_or(0));
        out.push(format!("class Class{letter}aaaaa {{\npublic:\n"));
        for _ in 0..*count {
            let initial = char::from(b'A' + u8::try_from(number % 26).unwrap_or(0));
            out.push(format!(
                "\tinline void Function{initial}{}() {{}}\n",
                suffix(number / 26, 7)
            ));
            number += 1;
        }
        out.push("};\n\n".to_owned());
    }
    out.concat()
}

/// A fixed-width base-26 lowercase suffix.
fn suffix(mut value: u32, width: usize) -> String {
    let mut letters = vec![b'a'; width];
    for slot in letters.iter_mut().rev() {
        *slot = b'a' + u8::try_from(value % 26).unwrap_or(0);
        value /= 26;
    }
    String::from_utf8(letters).unwrap_or_default()
}
