#![cfg(test)]
//! Behaviour of the `rbit` binary, driven the way people and agents drive it:
//! command lines in, exit codes and `--json` out. Every test gets a private
//! tool home and a catalog whose only compiler is a local script, so no test
//! touches the network or the user's installed tools.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

mod rewrite;

/// A scratch world: tool home, test catalog, and one project directory.
pub(crate) struct World {
    temp: TempDir,
}

/// A finished `rbit` run.
pub(crate) struct Run {
    code: i32,
    json: Value,
    stderr: String,
}

const PROJECT: &str = r#"
name = "demo"

[toolchain]
compiler = "fake-cc"

[[step]]
id = "compile.main"
program = "cc"
args = ["${SOURCE}/main.c", "app.bin"]
inputs = ["source/main.c"]
outputs = ["app.bin"]

[[target]]
id = "app"
artifact = "app.bin"
reference = "reference/app.bin"
"#;

/// The fake compiler's file: a batch file on Windows, a shell script elsewhere.
const COMPILER: &str = if cfg!(windows) { "cc.cmd" } else { "cc" };

impl World {
    /// A world whose catalog pins the fake compiler with `sha256`, or with
    /// its real digest when `None`.
    fn with_pin(sha256: Option<&str>) -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let mirror = temp.path().join("mirror");
        fs::create_dir_all(&mirror).expect("mirror dir");
        // Copies its last two arguments, so forced-include flags change nothing.
        let script: &[u8] = if cfg!(windows) {
            concat!(
                "@echo off\r\n:next\r\nif not \"%~3\"==\"\" (shift & goto next)\r\n",
                // `copy` reads `/` as a switch; compilers take either separator.
                "set \"from=%~1\"\r\nset \"to=%~2\"\r\ncopy /b /y \"%from:/=\\%\" \"%to:/=\\%\" >nul\r\n",
            )
            .as_bytes()
        } else {
            b"#!/bin/sh\nwhile [ $# -gt 2 ]; do shift; done\ncp \"$1\" \"$2\"\n"
        };
        fs::write(mirror.join(COMPILER), script).expect("compiler script");
        let pin = sha256.map_or_else(|| hex::encode(Sha256::digest(script)), str::to_owned);
        let catalog = format!(
            "[tools.fake-cc]\nkind = \"compiler\"\nsummary = \"Copies its input\"\nversion = \"1\"\nruns = \"host\"\nbin = {{ cc = \"{COMPILER}\" }}\nforce-include = [\"-include\", \"{{}}\"]\n\
             [[tools.fake-cc.get]]\nurl = {url:?}\nsha256 = \"{pin}\"\nas = \"{COMPILER}\"\n",
            url = mirror.join(COMPILER).display().to_string(),
        );
        fs::write(temp.path().join("catalog.toml"), catalog).expect("catalog");
        let project = temp.path().join("project");
        fs::create_dir_all(project.join("reference")).expect("project dir");
        fs::write(project.join("reprobit.toml"), PROJECT).expect("project file");
        fs::write(project.join("main.c"), "original program\n").expect("source");
        fs::write(project.join("reference/app.bin"), "original program\n").expect("reference");
        Self { temp }
    }

    fn new() -> Self {
        Self::with_pin(None)
    }

    fn project(&self) -> PathBuf {
        self.temp.path().join("project")
    }

    fn home(&self) -> PathBuf {
        self.temp.path().join("home")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rbit"));
        command
            .env("REPROBIT_HOME", self.home())
            .env("REPROBIT_CATALOGS", self.temp.path().join("catalog.toml"))
            .current_dir(self.project());
        command
    }

    /// Runs `rbit --json <args>` in the project directory.
    fn rbit(&self, args: &[&str]) -> Run {
        let output = self
            .command()
            .arg("--json")
            .args(args)
            .output()
            .expect("rbit runs");
        Run {
            code: output.status.code().unwrap_or(-1),
            json: serde_json::from_slice(&output.stdout).unwrap_or(Value::Null),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    fn edit(&self, relative: &str, text: &str) {
        fs::write(self.project().join(relative), text).expect("edit");
    }
}

fn succeeds(run: &Run) -> &Value {
    assert_eq!(run.code, 0, "rbit failed: {}\n{}", run.json, run.stderr);
    &run.json
}

#[test]
fn installed_tools_live_under_the_home_and_can_be_removed() {
    let world = World::new();
    let installed = succeeds(&world.rbit(&["tool", "install", "fake-cc"]))[0]["path"]
        .as_str()
        .expect("path")
        .to_owned();
    assert!(Path::new(&installed).starts_with(world.home()));
    let which = succeeds(&world.rbit(&["tool", "which", "fake-cc", "cc"]))["path"]
        .as_str()
        .expect("path")
        .to_owned();
    assert!(Path::new(&which).is_file());
    assert_eq!(
        succeeds(&world.rbit(&["tool", "remove", "fake-cc"]))["removed"],
        json!(["fake-cc"])
    );
    assert_eq!(world.rbit(&["tool", "which", "fake-cc"]).code, 2);

    succeeds(&world.rbit(&["tool", "install", "fake-cc"]));
    succeeds(&world.rbit(&["tool", "remove", "--all"]));
    assert!(!world.home().join("tools").exists() && !world.home().join("downloads").exists());
}

#[test]
fn a_command_nested_in_an_app_bundle_is_found_by_its_name() {
    let world = World::new();
    let catalog = world.temp.path().join("catalog.toml");
    let text = fs::read_to_string(&catalog).expect("catalog");
    let nested = text.replace(
        &format!("as = \"{COMPILER}\""),
        &format!("as = \"Fake.app/Contents/MacOS/{COMPILER}\""),
    );
    fs::write(&catalog, nested).expect("catalog");

    succeeds(&world.rbit(&["tool", "install", "fake-cc"]));
    let which = succeeds(&world.rbit(&["tool", "which", "fake-cc", "cc"]))["path"]
        .as_str()
        .expect("path")
        .to_owned();

    assert!(
        Path::new(&which).ends_with(format!("Fake.app/Contents/MacOS/{COMPILER}")),
        "{which}"
    );
}

#[test]
fn a_download_with_the_wrong_digest_is_refused() {
    let world = World::with_pin(Some(&"0".repeat(64)));
    let run = world.rbit(&["tool", "install", "fake-cc"]);
    assert_eq!(run.code, 2);
    assert!(
        run.json["error"]
            .as_str()
            .is_some_and(|error| error.contains("SHA-256")),
        "{}",
        run.json
    );
    assert_eq!(
        succeeds(&world.rbit(&["tool", "list", "--installed"])),
        &json!([])
    );
}

#[test]
fn the_catalog_lists_and_describes_tools() {
    let world = World::new();
    let listed = succeeds(&world.rbit(&["tool", "list", "copies its input"])).clone();
    assert_eq!(listed.as_array().map(Vec::len), Some(1));
    assert_eq!(listed[0]["id"], "fake-cc");
    let built_in = succeeds(&world.rbit(&["tool", "list", "objdiff"])).clone();
    assert!(built_in.as_array().is_some_and(|rows| !rows.is_empty()));
    let unknown = world.rbit(&["tool", "info", "fake-c"]);
    assert_eq!(unknown.code, 2);
    assert!(
        unknown.json["error"]
            .as_str()
            .is_some_and(|error| error.contains("fake-cc")),
        "suggests the close id: {}",
        unknown.json
    );
}

#[test]
fn init_writes_a_project_and_ignores_its_state() {
    let world = World::new();
    let fresh = world.temp.path().join("fresh");
    let dir = fresh.to_string_lossy().into_owned();
    succeeds(&world.rbit(&["init", &dir, "--compiler", "fake-cc"]));
    let project = fs::read_to_string(fresh.join("reprobit.toml")).expect("project file");
    assert!(project.contains("compiler = \"fake-cc\"") && project.contains("name = \"fresh\""));
    assert_eq!(
        fs::read_to_string(fresh.join(".gitignore")).expect("ignore file"),
        "/.reprobit/\n"
    );
    assert_eq!(
        world.rbit(&["init", &dir, "--compiler", "fake-cc"]).code,
        2,
        "refuses to overwrite"
    );
}

#[test]
fn status_names_the_next_step_until_verification_passes() {
    let world = World::new();
    let before = world.rbit(&["status"]);
    assert_eq!(before.code, 1);
    assert!(
        before.json["next"]
            .as_str()
            .is_some_and(|next| next.contains("compiler")),
        "{}",
        before.json
    );
    assert_eq!(
        succeeds(&world.rbit(&["setup"]))["status"]["next"],
        "`rbit verify`"
    );
    succeeds(&world.rbit(&["verify"]));
    assert_eq!(succeeds(&world.rbit(&["status"]))["verified"], true);
}

#[test]
fn verify_passes_on_a_match_and_reports_where_a_mismatch_is() {
    let world = World::new();
    succeeds(&world.rbit(&["setup"]));
    let passed = succeeds(&world.rbit(&["verify"])).clone();
    assert_eq!(passed["identical"], true);
    assert_eq!(passed["compiler"]["id"], "fake-cc");
    let reports = world.project().join(".reprobit/reports");
    assert!(reports.join("report.json").is_file() && reports.join("report.html").is_file());

    world.edit("main.c", "original pr0gram\n");
    let failed = world.rbit(&["verify"]);
    assert_eq!(failed.code, 1);
    assert_eq!(failed.json["targets"][0]["identical"], false);
    assert_eq!(failed.json["targets"][0]["differences"][0]["offset"], 11);
    let diff = world.rbit(&["diff"]);
    assert_eq!(diff.code, 1, "diff after a failed verify: {}", diff.json);
    assert_eq!(diff.json[0]["differences"][0]["offset"], 11);
}

#[test]
fn build_reuses_unchanged_steps_and_diff_compares_the_last_build() {
    let world = World::new();
    succeeds(&world.rbit(&["setup"]));
    assert_eq!(
        succeeds(&world.rbit(&["build"]))["steps"][0]["cached"],
        false
    );
    assert_eq!(
        succeeds(&world.rbit(&["build"]))["steps"][0]["cached"],
        true
    );
    succeeds(&world.rbit(&["diff"]));

    world.edit("main.c", "changed\n");
    assert_eq!(
        succeeds(&world.rbit(&["build"]))["steps"][0]["cached"],
        false
    );
    let diff = world.rbit(&["diff"]);
    assert_eq!(diff.code, 1);
    assert_eq!(diff.json[0]["identical"], false);
}

#[test]
fn repair_drops_a_prelude_only_when_the_build_still_matches() {
    let world = World::new();
    let project = fs::read_to_string(world.project().join("reprobit.toml")).expect("project");
    let with_prelude = project.replace(
        "outputs = [\"app.bin\"]",
        "outputs = [\"app.bin\"]\nprelude = { family = \"forward\", count = 2 }",
    );
    world.edit("reprobit.toml", &with_prelude);
    succeeds(&world.rbit(&["setup"]));

    world.edit("main.c", "edited program\n");
    let mismatched = world.rbit(&["repair"]);
    assert_eq!(mismatched.code, 1, "{}", mismatched.json);
    assert_eq!(mismatched.json["kept"], json!(["compile.main"]));

    world.edit("main.c", "original program\n");
    let repaired = world.rbit(&["repair"]);
    assert_eq!(succeeds(&repaired)["dropped"], json!(["compile.main"]));
    let saved = fs::read_to_string(world.project().join("reprobit.toml")).expect("project");
    assert!(!saved.contains("prelude"), "{saved}");
    succeeds(&world.rbit(&["verify"]));
}

/// A big-endian `PowerPC` Mach-O executable whose only load command is `LC_UUID`.
fn powerpc_image(uuid: u8) -> Vec<u8> {
    let mut image = Vec::new();
    for word in [0xfeed_face_u32, 18, 0, 2, 1, 24, 0, 0x1b, 24] {
        image.extend(word.to_be_bytes());
    }
    image.extend([uuid; 16]);
    image
}

#[test]
fn build_id_normalization_ignores_only_the_link_identity() {
    let world = World::new();
    fs::write(world.project().join("main.c"), powerpc_image(1)).expect("built image");
    fs::write(world.project().join("reference/app.bin"), powerpc_image(2)).expect("reference");
    succeeds(&world.rbit(&["setup"]));
    assert_eq!(world.rbit(&["verify"]).code, 1);

    let project = fs::read_to_string(world.project().join("reprobit.toml")).expect("project");
    world.edit(
        "reprobit.toml",
        &format!("{project}normalize = [\"build-id\"]\n"),
    );
    let normalized = world.rbit(&["verify"]);
    assert_eq!(succeeds(&normalized)["targets"][0]["normalized_fields"], 2);

    let mut changed_code = powerpc_image(1);
    changed_code[4] = 7;
    fs::write(world.project().join("main.c"), changed_code).expect("built image");
    assert_eq!(world.rbit(&["verify"]).code, 1);
}

/// A section-less 32-bit PE image linked at `timestamp`.
fn pe_image(timestamp: u32) -> Vec<u8> {
    let mut image = vec![0; 0x40 + 24 + 96];
    image[..2].copy_from_slice(b"MZ");
    image[0x3c] = 0x40;
    image[0x40..0x44].copy_from_slice(b"PE\0\0");
    image[0x44..0x46].copy_from_slice(&0x14c_u16.to_le_bytes());
    image[0x48..0x4c].copy_from_slice(&timestamp.to_le_bytes());
    image[0x54] = 96;
    image[0x58..0x5a].copy_from_slice(&0x10b_u16.to_le_bytes());
    image
}

#[test]
fn a_stamped_link_time_makes_the_artifact_itself_match() {
    let world = World::new();
    fs::write(world.project().join("main.c"), pe_image(1)).expect("built image");
    fs::write(
        world.project().join("reference/app.bin"),
        pe_image(0x3f3a_5c1e),
    )
    .expect("reference");
    succeeds(&world.rbit(&["setup"]));
    assert_eq!(world.rbit(&["verify"]).code, 1);

    let project = fs::read_to_string(world.project().join("reprobit.toml")).expect("project");
    world.edit(
        "reprobit.toml",
        &format!("{project}stamp = {{ pe-timestamp = 0x3f3a5c1e }}\n"),
    );
    let verified = world.rbit(&["verify"]);
    let target = &succeeds(&verified)["targets"][0];
    assert_eq!(target["artifact_sha256"], target["reference_sha256"]);
    assert_eq!(target["stamped"]["pe-timestamp"], 0x3f3a_5c1e);

    world.edit(
        "reprobit.toml",
        &format!("{project}stamp = {{ pe-timestamp = 0x3f3a5c1f }}\n"),
    );
    assert_eq!(world.rbit(&["verify"]).code, 1);
}

#[test]
fn steps_imported_from_a_compilation_database_build_and_verify() {
    let world = World::new();
    let root = world.project().canonicalize().expect("root");
    let database = json!([{
        "directory": root.join("out"),
        "file": root.join("main.c"),
        "arguments": [
            "/usr/bin/cc",
            format!("-I{}/include", root.display()),
            root.join("main.c"),
            root.join("out/app.bin"),
        ],
        "output": root.join("out/app.bin"),
    }]);
    world.edit("compile_commands.json", &database.to_string());

    let imported = world
        .command()
        .args(["import", "compile_commands.json"])
        .output()
        .expect("rbit runs");
    assert!(imported.status.success());
    let steps = String::from_utf8(imported.stdout).expect("toml");
    let project = format!(
        "name = \"demo\"\n[toolchain]\ncompiler = \"fake-cc\"\n\n{steps}\n\
         [[target]]\nid = \"app\"\nartifact = \"app.bin\"\nreference = \"reference/app.bin\"\n"
    );
    world.edit("reprobit.toml", &project);

    succeeds(&world.rbit(&["setup"]));
    succeeds(&world.rbit(&["verify"]));
    assert!(!project.contains(&root.display().to_string()), "{project}");
}

#[test]
fn a_compiler_that_changed_since_setup_is_refused() {
    let world = World::new();
    succeeds(&world.rbit(&["setup"]));
    assert!(
        fs::read_to_string(world.project().join("reprobit.lock"))
            .expect("lock")
            .contains("fake-cc")
    );
    let which = succeeds(&world.rbit(&["tool", "which", "fake-cc", "cc"]))["path"]
        .as_str()
        .expect("path")
        .to_owned();
    fs::write(&which, "#!/bin/sh\necho tampered > \"$2\"\n").expect("tamper");
    let run = world.rbit(&["build"]);
    assert_eq!(run.code, 2);
    assert!(
        run.json["error"]
            .as_str()
            .is_some_and(|error| error.contains("reprobit.lock")),
        "{}",
        run.json
    );
}

#[test]
fn a_step_may_not_read_a_reference() {
    let world = World::new();
    world.edit(
        "reprobit.toml",
        &PROJECT.replace(
            "inputs = [\"source/main.c\"]",
            "inputs = [\"source/reference/app.bin\"]",
        ),
    );
    succeeds(&world.rbit(&["setup"]));
    let run = world.rbit(&["build"]);
    assert_eq!(run.code, 2);
    assert!(
        run.json["error"]
            .as_str()
            .is_some_and(|error| error.contains("reference")),
        "{}",
        run.json
    );
}

#[test]
fn a_reference_that_does_not_match_its_pin_is_reported() {
    let world = World::new();
    world.edit(
        "reprobit.toml",
        &PROJECT.replace(
            "reference = \"reference/app.bin\"",
            &format!(
                "reference = \"reference/app.bin\"\nsha256 = \"{}\"",
                "0".repeat(64)
            ),
        ),
    );
    let run = world.rbit(&["setup"]);
    assert_eq!(run.code, 1);
    let checks = run.json["status"]["checks"]
        .as_array()
        .expect("checks")
        .clone();
    let reference = checks
        .iter()
        .find(|check| check["name"] == "reference app")
        .expect("reference check");
    assert_eq!(reference["ok"], false);
}

#[test]
fn the_published_schemas_are_the_ones_rbit_reads() {
    let world = World::new();
    let published = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
    for name in ["project", "catalog"] {
        let schema = succeeds(&world.rbit(&["schema", name])).clone();
        let file = fs::read_to_string(published.join(format!("{name}.schema.json")))
            .expect("published schema");
        let file: Value = serde_json::from_str(&file).expect("schema is JSON");
        assert_eq!(
            schema, file,
            "schemas/{name}.schema.json is stale; regenerate it with `rbit schema {name}`"
        );
    }
    assert_eq!(world.rbit(&["schema", "lockfile"]).code, 2);
}

#[test]
fn the_mcp_server_answers_both_handshakes_and_runs_commands() {
    let world = World::new();
    let mut server = world
        .command()
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("server starts");
    let mut input = server.stdin.take().expect("stdin");
    let mut output = BufReader::new(server.stdout.take().expect("stdout"));
    let mut ask = |id: u32, method: &str, params: Value| -> Value {
        writeln!(
            input,
            "{}",
            json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
        )
        .expect("request");
        let mut line = String::new();
        output.read_line(&mut line).expect("reply");
        let reply: Value = serde_json::from_str(&line).expect("reply is JSON");
        assert_eq!(reply["id"], id);
        reply
    };

    assert_eq!(
        ask(1, "initialize", json!({ "protocolVersion": "2025-06-18" }))["result"]["protocolVersion"],
        "2025-06-18"
    );
    assert!(
        ask(2, "server/discover", json!({}))["result"]["supportedVersions"]
            .as_array()
            .is_some_and(|versions| versions.contains(&json!("2026-07-28")))
    );
    let listed = ask(3, "tools/list", json!({}));
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(names.contains(&"verify") && names.contains(&"tool_install"));

    let dir = world.project().to_string_lossy().into_owned();
    let setup = ask(
        4,
        "tools/call",
        json!({ "name": "setup", "arguments": { "dir": dir } }),
    );
    assert_eq!(setup["result"]["isError"], false, "{setup}");
    let verify = ask(
        5,
        "tools/call",
        json!({ "name": "verify", "arguments": { "dir": dir } }),
    );
    assert_eq!(
        verify["result"]["structuredContent"]["identical"], true,
        "{verify}"
    );
    assert!(ask(6, "nonsense", json!({}))["error"].is_object());

    drop(input);
    assert!(server.wait().expect("server exits").success());
}
