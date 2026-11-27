//! Running producer commands so they see the same paths on every machine.
//!
//! A stage directory holds a copy of the declared sources, the build output,
//! and a temporary directory; the compiler is attached as `toolchain`. The
//! whole stage appears to the compiler under the project's logical roots: a
//! private Wine drive, a `subst` drive on Windows, or a fixed mount inside a
//! container. A compiler that writes paths into its output therefore writes
//! the same bytes everywhere, and since only declared inputs are copied in,
//! it can never read the reference binary.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{Context, Result, bail, ensure};

use crate::project::Paths;
use crate::tools::{self, on_path};
use crate::ui;

/// The image `docker` runs Win32 compilers in, built on first use. Wine saves
/// the registry only when wineserver exits, so the build waits for it; a
/// prefix without one is set up again by every container, racing the compiler.
pub(crate) const WINE_IMAGE: &str = "reprobit-wine:2";
const WINE_DOCKERFILE: &str = "FROM i386/debian:bookworm-slim\n\
RUN apt-get update && apt-get install -y --no-install-recommends wine wine32 && rm -rf /var/lib/apt/lists/*\n\
ENV WINEDEBUG=-all WINEPREFIX=/wine\n\
RUN wineboot --init >/dev/null 2>&1 && wineserver -w && test -f /wine/system.reg && rm -f /wine/dosdevices/z:\n";
/// The image `docker` runs Linux compilers in, x86-64 and i386 alike. Pinned
/// by index digest so every machine runs the same loader and libc.
const LINUX_IMAGE: &str =
    "debian:bookworm-slim@sha256:e67eeead736ff7bbe258ef088bff683635c561b1fcfefad0806c1df4ab49ce0e";
/// Where the stage is mounted inside a container.
const MOUNT: &str = "/r";

/// How one command is launched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Directly on this machine, with host paths.
    Host,
    /// Under Wine with a private prefix whose only extra drive is the stage.
    Wine(PathBuf),
    /// Under wibo, the small Win32 loader decomp.me uses.
    Wibo(PathBuf),
    /// In a container through `docker` (`OrbStack`, Docker Desktop, `Colima`) or `podman`.
    Docker {
        cli: PathBuf,
        windows: bool,
        platform: &'static str,
    },
    /// On Windows, with the stage mounted as a drive letter by `subst`.
    Subst,
}

impl Mode {
    /// Picks how to run a tool that `runs` on the given platform.
    /// `choice` is the project's `runner` setting.
    pub(crate) fn pick(runs: &str, choice: &str) -> Result<Self> {
        let docker = || {
            // Win32 programs run under i386 Wine, so they share the i386 platform.
            let platform = if matches!(runs, "windows" | "linux-i386") {
                "linux/386"
            } else {
                "linux/amd64"
            };
            on_path("docker")
                .or_else(|| on_path("podman"))
                .map(|cli| Self::Docker {
                    cli,
                    windows: runs == "windows",
                    platform,
                })
        };
        let wibo = || {
            let catalog = crate::catalog::Catalog::load().ok()?;
            let installed = tools::find(&catalog, "wibo").ok()??;
            tools::command(&installed, None).ok().map(Self::Wibo)
        };
        let picked = match (choice, runs) {
            ("native", _) | (_, "host") => Some(Self::Host),
            ("wine", _) => on_path("wine").map(Self::Wine),
            ("wibo", _) => wibo(),
            ("docker", _) => docker(),
            ("auto", "windows") if cfg!(windows) => Some(Self::Subst),
            ("auto", "windows") => on_path("wine")
                .map(Self::Wine)
                .or_else(wibo)
                .or_else(docker),
            ("auto", "linux-x86_64" | "linux-i386")
                if cfg!(all(target_os = "linux", target_arch = "x86_64")) =>
            {
                Some(Self::Host)
            }
            ("auto", "linux-x86_64" | "linux-i386") => docker(),
            ("auto", other) => bail!("unknown platform `{other}` for a tool's `runs`"),
            (other, _) => {
                bail!("unknown runner `{other}`; use auto, native, wine, wibo, or docker")
            }
        };
        picked.with_context(|| match runs {
            "windows" => "this tool is a Win32 program: install Wine (`brew install --cask wine-stable`, \
                          `apt install wine`), run `rbit tool install wibo`, or start Docker/OrbStack"
                .to_owned(),
            _ => format!("this tool runs on {runs}: start Docker or OrbStack, or use a {runs} machine"),
        })
    }

    /// A short name for reports.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Host => "native",
            Self::Wine(_) => "wine",
            Self::Wibo(_) => "wibo",
            Self::Docker { .. } => "docker",
            Self::Subst => "subst",
        }
    }

    /// Whether the compiler sees DOS paths such as `R:\\build`.
    pub(crate) fn dos(&self) -> bool {
        matches!(
            self,
            Self::Wine(_) | Self::Subst | Self::Docker { windows: true, .. }
        )
    }
}

/// A prepared stage for one build.
#[derive(Debug)]
pub(crate) struct Stage {
    /// Host directory holding `build/` and the temporary directory.
    pub(crate) root: PathBuf,
    toolchain: PathBuf,
    paths: Paths,
    mode: Mode,
    logs: PathBuf,
}

/// The part of a logical root below its drive or leading slash: `R:\source`
/// becomes `source`.
fn below_root(logical: &str) -> String {
    let rest = logical.split_once(':').map_or(logical, |(_, rest)| rest);
    rest.trim_start_matches(['\\', '/']).replace('\\', "/")
}

fn drive(paths: &Paths) -> Result<char> {
    let letter = paths
        .source
        .chars()
        .next()
        .filter(|c| c.is_ascii_alphabetic() && paths.source[1..].starts_with(':'));
    let letter = letter.context("Win32 compilers need DOS logical paths such as R:\\source")?;
    for path in [&paths.build, &paths.toolchain, &paths.temp] {
        ensure!(
            path.to_ascii_uppercase()
                .starts_with(&format!("{}:", letter.to_ascii_uppercase())),
            "all logical paths must share one drive"
        );
    }
    Ok(letter.to_ascii_lowercase())
}

impl Stage {
    /// Creates `root` with an empty build tree, copies `sources` (project
    /// paths relative to `project`) into it, and attaches the toolchain.
    pub(crate) fn create(
        root: &Path,
        project: &Path,
        sources: &[String],
        toolchain: &Path,
        paths: &Paths,
        mode: Mode,
        state: &Path,
    ) -> Result<Self> {
        fs::create_dir_all(root)?;
        for logical in [&paths.build, &paths.temp] {
            fs::create_dir_all(root.join(below_root(logical)))?;
        }
        let source = root.join(below_root(&paths.source));
        if source.symlink_metadata().is_ok() {
            fs::remove_dir_all(&source)?;
        }
        for relative in sources {
            let to = source.join(relative);
            fs::create_dir_all(to.parent().context("source path")?)?;
            fs::copy(project.join(relative), &to)
                .with_context(|| format!("cannot stage {relative}"))?;
        }
        let attached = root.join(below_root(&paths.toolchain));
        if matches!(mode, Mode::Docker { .. }) {
            // The container mounts the toolchain there; a link or mount point
            // left by an earlier run must not join the streamed stage.
            unlink(&attached)?;
        } else {
            link(toolchain, &attached)?;
        }
        let stage = Self {
            root: root.to_path_buf(),
            toolchain: toolchain.to_path_buf(),
            paths: paths.clone(),
            mode,
            logs: state.join("logs"),
        };
        if let Mode::Wine(_) = &stage.mode {
            stage.wine_prefix(state)?;
        }
        if let Mode::Docker {
            cli, windows: true, ..
        } = &stage.mode
        {
            ensure_image(cli)?;
        }
        Ok(stage)
    }

    /// The host directory behind the build root.
    pub(crate) fn build(&self) -> PathBuf {
        self.root.join(below_root(&self.paths.build))
    }

    /// The runner in use.
    pub(crate) fn mode(&self) -> &Mode {
        &self.mode
    }

    /// The compiler-visible spelling of the root named `name` (`SOURCE`,
    /// `BUILD`, `TOOLCHAIN`, or `TEMP`).
    fn logical(&self, name: &str, mode: &Mode) -> String {
        let logical = match name {
            "SOURCE" => &self.paths.source,
            "BUILD" => &self.paths.build,
            "TOOLCHAIN" => &self.paths.toolchain,
            _ => &self.paths.temp,
        };
        if mode.dos() {
            return logical.clone();
        }
        let below = below_root(logical);
        if matches!(mode, Mode::Docker { .. }) {
            return format!("{MOUNT}/{below}");
        }
        // Relative to the build directory, the working directory of every
        // step: identical on every machine, and never mistaken for a `/x`
        // switch by DOS-style command lines.
        let depth = below_root(&self.paths.build)
            .split('/')
            .filter(|part| !part.is_empty())
            .count();
        let up = vec![".."; depth].join("/");
        if below.is_empty() {
            up
        } else {
            format!("{up}/{below}")
        }
    }

    /// Expands `${SOURCE}`, `${BUILD}`, `${TOOLCHAIN}`, `${TEMP}`, and
    /// `${TOOLCHAIN_ROOTED}` (the toolchain root without its drive, the
    /// spelling classic MSVC environments use).
    pub(crate) fn expand(&self, text: &str, mode: &Mode) -> String {
        let toolchain = self.logical("TOOLCHAIN", mode);
        let rooted = toolchain
            .split_once(':')
            .map_or(toolchain.as_str(), |(_, rest)| rest)
            .to_owned();
        text.replace("${TOOLCHAIN_ROOTED}", &rooted)
            .replace("${SOURCE}", &self.logical("SOURCE", mode))
            .replace("${BUILD}", &self.logical("BUILD", mode))
            .replace("${TOOLCHAIN}", &toolchain)
            .replace("${TEMP}", &self.logical("TEMP", mode))
    }

    /// Runs `program` (a host path inside the toolchain, or any host tool when
    /// `inside_toolchain` is false) and returns its combined log.
    pub(crate) fn run(
        &self,
        id: &str,
        program: &Path,
        inside_toolchain: bool,
        args: &[String],
        env: &BTreeMap<String, String>,
    ) -> Result<String> {
        // Other catalog tools are host programs, so they get host paths.
        let mode = if inside_toolchain {
            &self.mode
        } else {
            &Mode::Host
        };
        let args: Vec<String> = args.iter().map(|arg| self.expand(arg, mode)).collect();
        let mut env: BTreeMap<String, String> = env
            .iter()
            .map(|(key, value)| (key.clone(), self.expand(value, mode)))
            .collect();
        let relative = program
            .strip_prefix(&self.toolchain)
            .ok()
            .filter(|_| inside_toolchain);
        let program_logical = relative.map_or_else(
            || program.to_string_lossy().into_owned(),
            |relative| {
                let joined = relative.to_string_lossy().replace('\\', "/");
                let root = self.logical("TOOLCHAIN", mode);
                if mode.dos() {
                    format!("{root}\\{}", joined.replace('/', "\\"))
                } else {
                    format!("{root}/{joined}")
                }
            },
        );
        let build = self.build();
        let mut command = match mode {
            Mode::Host => {
                let mut command = tools::launch(program);
                command.args(&args).current_dir(build);
                command
            }
            Mode::Subst => {
                let mut command = Command::new(&program_logical);
                command.args(&args).current_dir(&self.paths.build);
                command
            }
            Mode::Wine(wine) => {
                if let Some(path) = env.get("PATH").cloned() {
                    env.insert("WINEPATH".into(), path);
                    env.remove("PATH");
                }
                env.entry("WINEDEBUG".into())
                    .or_insert_with(|| "-all".into());
                env.insert(
                    "WINEPREFIX".into(),
                    self.prefix().to_string_lossy().into_owned(),
                );
                let mut command = Command::new(wine);
                command
                    .arg(&program_logical)
                    .args(&args)
                    .current_dir(&build);
                command
            }
            Mode::Wibo(wibo) => {
                let mut command = Command::new(wibo);
                command.arg(program).args(&args).current_dir(&build);
                command
            }
            Mode::Docker {
                cli,
                windows,
                platform,
            } => self.docker(cli, *windows, platform, &program_logical, &args, &env),
        };
        if !matches!(mode, Mode::Docker { .. }) {
            command.envs(&env);
        }
        let _drive = if inside_toolchain {
            self.subst()?
        } else {
            None
        };
        fs::create_dir_all(&self.logs)?;
        let output = if matches!(mode, Mode::Docker { .. }) {
            self.contained(command)
        } else {
            command.stdin(Stdio::null()).output().map_err(Into::into)
        }
        .with_context(|| format!("cannot start {program_logical}"))?;
        let log = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        fs::write(self.logs.join(format!("{id}.log")), &log)?;
        if !output.status.success() {
            let lines: Vec<&str> = log.lines().collect();
            bail!(
                "step {id} failed ({}) under {}:\n{}",
                output.status,
                mode.name(),
                lines[lines.len().saturating_sub(20)..].join("\n")
            );
        }
        Ok(log)
    }

    /// Runs a container that reads the stage as a tar on stdin, writes the
    /// build tree back as a tar on stdout, and logs on stderr. VM file
    /// sharing on macOS can show a container a stale view of a
    /// host directory that changed between runs, so the writable stage never
    /// crosses a bind mount.
    fn contained(&self, mut command: Command) -> Result<Output> {
        let mut stage = tar::Builder::new(Vec::new());
        stage.append_dir_all(".", &self.root)?;
        let stage = stage.into_inner()?;
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdin = child.stdin.take().context("container stdin")?;
        let writer = std::thread::spawn(move || stdin.write_all(&stage));
        let mut output = child.wait_with_output()?;
        let written = writer
            .join()
            .map_err(|_| anyhow::anyhow!("the stage writer panicked"))?;
        // A container that failed may stop reading early; its status says why.
        if output.status.success() {
            written?;
        }
        if !output.stdout.is_empty() {
            tar::Archive::new(output.stdout.as_slice()).unpack(&self.root)?;
        }
        output.stdout.clear();
        Ok(output)
    }

    fn docker(
        &self,
        cli: &Path,
        windows: bool,
        platform: &str,
        program: &str,
        args: &[String],
        env: &BTreeMap<String, String>,
    ) -> Command {
        let mut command = Command::new(cli);
        let mount = |host: &Path, inside: String, mode: &str| {
            format!("{}:{MOUNT}/{inside}:{mode}", host.display())
        };
        command.args(["run", "--rm", "--platform", platform]);
        command.arg("-i");
        command.arg("-v").arg(mount(
            &self.toolchain,
            below_root(&self.paths.toolchain),
            "ro",
        ));
        for (key, value) in env {
            let key = if windows && key == "PATH" {
                "WINEPATH"
            } else {
                key
            };
            command.arg("-e").arg(format!("{key}={value}"));
        }
        // `$0` is the build tree below the mount; the program's output goes
        // to stderr because stdout carries the build tree back.
        let launch = if windows {
            let letter = drive(&self.paths).unwrap_or('r');
            command.arg(WINE_IMAGE);
            format!("ln -sfn {MOUNT} /wine/dosdevices/{letter}: && wine")
        } else {
            command.arg(LINUX_IMAGE);
            String::new()
        };
        command.args(["sh", "-c"]).arg(format!(
            "tar -x -f - -C {MOUNT} && cd \"{MOUNT}/$0\" && {launch} \"$@\" >&2; status=$?; \
             cd {MOUNT} && tar -c -f - \"$0\"; exit $status"
        ));
        command.arg(below_root(&self.paths.build));
        command.arg(program).args(args);
        command
    }

    fn prefix(&self) -> PathBuf {
        self.logs.with_file_name("wine")
    }

    /// Makes the project's Wine prefix once, then leaves it with exactly two
    /// drives: `c:` and the stage. Dropping `z:` hides the host filesystem.
    fn wine_prefix(&self, state: &Path) -> Result<()> {
        let Mode::Wine(wine) = &self.mode else {
            return Ok(());
        };
        let prefix = state.join("wine");
        let devices = prefix.join("dosdevices");
        if !devices.join("c:").exists() {
            ui::note("Creating a private Wine prefix (first run only)");
            let status = Command::new(wine)
                .args(["wineboot", "--init"])
                .env("WINEPREFIX", &prefix)
                .env("WINEDEBUG", "-all")
                .env(
                    "WINEDLLOVERRIDES",
                    "winemenubuilder.exe=d;mscoree=d;mshtml=d",
                )
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            ensure!(
                status.success() && devices.join("c:").exists(),
                "wineboot could not create {}",
                prefix.display()
            );
        }
        let letter = drive(&self.paths)?;
        for entry in fs::read_dir(&devices)? {
            let entry = entry?;
            if entry.file_name() != "c:" {
                fs::remove_file(entry.path()).or_else(|_| fs::remove_dir_all(entry.path()))?;
            }
        }
        link(&self.root, &devices.join(format!("{letter}:")))
    }

    /// Maps the stage to its drive letter for the life of the returned guard.
    fn subst(&self) -> Result<Option<SubstGuard>> {
        if !matches!(self.mode, Mode::Subst) {
            return Ok(None);
        }
        let drive = format!("{}:", drive(&self.paths)?.to_ascii_uppercase());
        // A mapping left by an interrupted run would block the letter.
        best_effort(Command::new("subst").args([&drive, "/D"]));
        let status = Command::new("subst")
            .arg(&drive)
            .arg(&self.root)
            .stdout(ui::child_output())
            .status()?;
        ensure!(
            status.success(),
            "subst could not map {drive}; is the letter in use?"
        );
        Ok(Some(SubstGuard(drive)))
    }

    /// Stops the stage's Wine server so no process outlives the build.
    pub(crate) fn finish(&self) {
        if let Mode::Wine(wine) = &self.mode {
            let server = wine.with_file_name("wineserver");
            let server = if server.is_file() {
                server
            } else {
                PathBuf::from("wineserver")
            };
            best_effort(
                Command::new(server)
                    .arg("-k")
                    .env("WINEPREFIX", self.prefix()),
            );
        }
    }
}

/// Removes a `subst` drive on drop.
#[derive(Debug)]
struct SubstGuard(String);

impl Drop for SubstGuard {
    fn drop(&mut self) {
        best_effort(Command::new("subst").args([&self.0, "/D"]));
    }
}

/// Runs a cleanup command whose failure leaves nothing worse than before,
/// so it is only reported.
fn best_effort(command: &mut Command) {
    if let Err(error) = command.stdout(Stdio::null()).stderr(Stdio::null()).status() {
        ui::note(&format!(
            "cleanup {} did not run: {error}",
            command.get_program().to_string_lossy()
        ));
    }
}

/// Removes the link, file, or empty directory at `at`, if any.
fn unlink(at: &Path) -> Result<()> {
    if at.symlink_metadata().is_ok() {
        fs::remove_file(at).or_else(|_| fs::remove_dir(at))?;
    }
    Ok(())
}

/// Points `at` to `target`: a symlink on Unix, a junction on Windows.
fn link(target: &Path, at: &Path) -> Result<()> {
    unlink(at)?;
    if let Some(parent) = at.parent() {
        fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, at)?;
    #[cfg(windows)]
    {
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(at)
            .arg(target)
            .stdout(Stdio::null())
            .status()?;
        ensure!(status.success(), "cannot link {}", at.display());
    }
    Ok(())
}

fn ensure_image(cli: &Path) -> Result<()> {
    let present = Command::new(cli)
        .args(["image", "inspect", WINE_IMAGE])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if present.success() {
        return Ok(());
    }
    ui::note(&format!(
        "Building the {WINE_IMAGE} container image (first run only)"
    ));
    let mut child = Command::new(cli)
        .args(["build", "--platform", "linux/386", "-t", WINE_IMAGE, "-"])
        .stdin(Stdio::piped())
        .stdout(ui::child_output())
        .spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        std::io::Write::write_all(stdin, WINE_DOCKERFILE.as_bytes())?;
    }
    drop(child.stdin.take());
    ensure!(
        child.wait()?.success(),
        "cannot build the {WINE_IMAGE} image"
    );
    Ok(())
}
