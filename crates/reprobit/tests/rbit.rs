#![cfg(test)]
//! Behaviour of the `rbit` binary, driven the way people and agents drive it:
//! command lines in, exit codes and `--json` out. Every test gets a private
//! tool home and a catalog whose only compiler is a local script, so no test
//! touches the network or the user's installed tools.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use object::{
    Architecture, BinaryFormat, Endianness, Object, ObjectSection, RelocationFlags, SectionKind,
    SymbolFlags, SymbolKind, SymbolScope, write,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

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

/// A debug section for [`i386_object`], with relocations to its functions.
struct DebugSection {
    name: &'static str,
    data: Vec<u8>,
    /// `(offset, function, relocation type)`.
    relocations: Vec<(u64, &'static str, RelocationFlags)>,
}

/// An i386 object whose `.text` holds `functions` in order, plus `debug`.
fn i386_object(
    format: BinaryFormat,
    functions: &[(&str, &[u8])],
    debug: Vec<DebugSection>,
) -> Vec<u8> {
    let mut object = write::Object::new(format, Architecture::I386, Endianness::Little);
    object.set_mangling(write::Mangling::None);
    let text = object.add_section(Vec::new(), b".text".to_vec(), SectionKind::Text);
    let mut symbols = Vec::new();
    for (name, code) in functions {
        let value = object.append_section_data(text, code, 1);
        let symbol = object.add_symbol(write::Symbol {
            name: name.as_bytes().to_vec(),
            value,
            size: code.len() as u64,
            kind: SymbolKind::Text,
            scope: SymbolScope::Linkage,
            weak: false,
            section: write::SymbolSection::Section(text),
            flags: SymbolFlags::None,
        });
        symbols.push((*name, symbol));
    }
    for section in debug {
        let id = object.add_section(
            Vec::new(),
            section.name.as_bytes().to_vec(),
            SectionKind::Debug,
        );
        object.append_section_data(id, &section.data, 4);
        for (offset, function, flags) in section.relocations {
            let symbol = symbols
                .iter()
                .find(|(name, _)| *name == function)
                .expect("function")
                .1;
            object
                .add_relocation(
                    id,
                    write::Relocation {
                        offset,
                        symbol,
                        addend: 0,
                        flags,
                    },
                )
                .expect("relocation");
        }
    }
    object.write().expect("object")
}

/// `push esi; push edi; mov esi, [esp+12]; mov edi, esi; add edi, 1;
/// mov eax, edi; pop edi; pop esi; ret`
const SAVES_ESI_FIRST: &[u8] = &[
    0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0x8b, 0xfe, 0x83, 0xc7, 0x01, 0x8b, 0xc7, 0x5f, 0x5e, 0xc3,
];
/// The same function with `esi` and `edi` exchanged.
const SAVES_EDI_FIRST: &[u8] = &[
    0x57, 0x56, 0x8b, 0x7c, 0x24, 0x0c, 0x8b, 0xf7, 0x83, 0xc6, 0x01, 0x8b, 0xc6, 0x5e, 0x5f, 0xc3,
];

/// A COFF object whose only function, `_f`, is `code`.
fn coff_function(code: &[u8]) -> Vec<u8> {
    i386_object(BinaryFormat::Coff, &[("_f", code)], Vec::new())
}

/// A world whose step "compiles" `main.c` to `artifact`, the target.
fn object_world(artifact: &str, built: &[u8], reference: &[u8]) -> World {
    let world = World::new();
    let project = fs::read_to_string(world.project().join("reprobit.toml")).expect("project");
    world.edit("reprobit.toml", &project.replace("app.bin", artifact));
    fs::write(world.project().join("main.c"), built).expect("built object");
    fs::write(world.project().join("reference").join(artifact), reference).expect("reference");
    world
}

/// Declares `registers` for the step's `function`.
fn rename(world: &World, artifact: &str, function: &str) {
    let project = fs::read_to_string(world.project().join("reprobit.toml")).expect("project");
    world.edit(
        "reprobit.toml",
        &project.replace(
            &format!("outputs = [\"{artifact}\"]"),
            &format!("outputs = [\"{artifact}\"]\nregisters = {{ {function} = {{ esi = \"edi\", edi = \"esi\" }} }}"),
        ),
    );
}

#[test]
fn grind_finds_a_register_rename_and_the_saved_rename_matches() {
    let world = object_world(
        "main.obj",
        &coff_function(SAVES_ESI_FIRST),
        &coff_function(SAVES_EDI_FIRST),
    );
    succeeds(&world.rbit(&["setup"]));
    assert_eq!(world.rbit(&["verify"]).code, 1);
    let grind = [
        "grind",
        "--step",
        "compile.main",
        "--symbol",
        "_f",
        "--reference-object",
        "reference/main.obj",
        "--family",
        "registers",
    ];

    let found = world.rbit(&grind);
    let accepted = world.rbit(&[&grind[..], &["--accept"]].concat());

    assert_eq!(
        succeeds(&found)["found"],
        json!({ "prelude": null, "registers": { "esi": "edi", "edi": "esi" } })
    );
    assert_eq!(succeeds(&accepted)["accepted"], true);
    succeeds(&world.rbit(&["verify"]));
    assert_eq!(succeeds(&world.rbit(&grind))["already_matches"], true);
}

#[test]
fn a_rename_that_would_miss_an_implicit_register_use_is_refused() {
    // `movsd` copies from [esi] to [edi] without naming either register.
    let world = object_world(
        "main.obj",
        &coff_function(&[0x56, 0x57, 0xa5, 0x5f, 0x5e, 0xc3]),
        &coff_function(&[0xc3]),
    );
    rename(&world, "main.obj", "_f");
    succeeds(&world.rbit(&["setup"]));

    let refused = world.rbit(&["build"]);

    assert_ne!(refused.code, 0);
    assert!(refused.stderr.contains("implicitly"), "{}", refused.stderr);
}

/// i386 call-frame information: a CIE, then one FDE per function, relocated to it.
fn eh_frame(entries: &[(&'static str, &[u8])]) -> DebugSection {
    let entry = |data: &mut Vec<u8>, mut body: Vec<u8>| {
        body.resize(body.len().next_multiple_of(4), 0);
        data.extend(u32::try_from(body.len()).expect("short").to_le_bytes());
        data.extend(body);
    };
    let mut data = Vec::new();
    // Version 1, "zR", alignments 1 and -4, return register 8, pc-relative
    // addresses; the CFA is esp + 4 and the return address sits below it.
    entry(
        &mut data,
        vec![
            0, 0, 0, 0, 1, b'z', b'R', 0, 1, 0x7c, 8, 1, 0x1b, 0x0c, 4, 4, 0x88, 1,
        ],
    );
    let mut relocations = Vec::new();
    for (function, instructions) in entries {
        let at = data.len();
        relocations.push((
            at as u64 + 8,
            *function,
            RelocationFlags::Elf {
                r_type: object::elf::R_386_PC32,
            },
        ));
        let mut body = u32::try_from(at + 4).expect("short").to_le_bytes().to_vec();
        body.extend([0, 0, 0, 0, 16, 0, 0, 0, 0]);
        body.extend(*instructions);
        entry(&mut data, body);
    }
    DebugSection {
        name: ".eh_frame",
        data,
        relocations,
    }
}

/// DWARF 4 entries: per function, a subprogram with one variable whose
/// location is the single operation `location`.
fn dwarf_info(functions: &[(&str, u8)]) -> [DebugSection; 2] {
    let abbreviations = vec![
        1, 0x11, 1, 0, 0, // compile unit
        2, 0x2e, 1, 0x03, 0x08, 0x12, 0x06, 0, 0, // subprogram: name, high_pc
        3, 0x34, 0, 0x02, 0x18, 0, 0, // variable: exprloc location
        0,
    ];
    let mut entries = vec![1];
    for (name, location) in functions {
        entries.push(2);
        entries.extend(name.as_bytes());
        entries.extend([0, 16, 0, 0, 0, 3, 1, *location, 0]);
    }
    entries.push(0);
    let mut info = u32::try_from(entries.len() + 7)
        .expect("short")
        .to_le_bytes()
        .to_vec();
    info.extend([4, 0, 0, 0, 0, 0, 4]);
    info.extend(entries);
    [
        DebugSection {
            name: ".debug_abbrev",
            data: abbreviations,
            relocations: Vec::new(),
        },
        DebugSection {
            name: ".debug_info",
            data: info,
            relocations: Vec::new(),
        },
    ]
}

#[test]
fn a_register_rename_carries_into_dwarf_locations_and_unwind_rules() {
    // `g` saves and keeps a variable in esi too; only `f` is renamed.
    let object = |code: &[u8], saves: &[u8], location: u8| {
        let mut debug = vec![eh_frame(&[("f", saves), ("g", &[0x41, 0x0e, 8, 0x86, 2])])];
        debug.extend(dwarf_info(&[("f", location), ("g", 0x56)]));
        i386_object(
            BinaryFormat::Elf,
            &[("f", code), ("g", &[0x56, 0x5e, 0xc3])],
            debug,
        )
    };
    // esi saved at CFA-8 and edi at CFA-12, and the reverse.
    let built = object(
        SAVES_ESI_FIRST,
        &[0x41, 0x0e, 8, 0x86, 2, 0x41, 0x0e, 12, 0x87, 3],
        0x56,
    );
    let reference = object(
        SAVES_EDI_FIRST,
        &[0x41, 0x0e, 8, 0x87, 2, 0x41, 0x0e, 12, 0x86, 3],
        0x57,
    );
    let world = object_world("main.o", &built, &reference);
    rename(&world, "main.o", "f");
    succeeds(&world.rbit(&["setup"]));

    let verified = world.rbit(&["verify"]);

    assert_eq!(succeeds(&verified)["identical"], true);
}

/// A C13 `.debug$S` in which every function keeps a variable in esi and
/// shares the frame program `$T0 $esi =`.
fn codeview(functions: &[&'static str]) -> DebugSection {
    let subsection = |data: &mut Vec<u8>, kind: u32, body: &[u8]| {
        data.extend(kind.to_le_bytes());
        data.extend(u32::try_from(body.len()).expect("short").to_le_bytes());
        let at = data.len();
        data.extend(body);
        data.resize(data.len().next_multiple_of(4), 0);
        at
    };
    let record = |records: &mut Vec<u8>, kind: u16, body: &[u8]| {
        records.extend(u16::try_from(body.len() + 2).expect("short").to_le_bytes());
        records.extend(kind.to_le_bytes());
        records.extend(body);
    };
    let (mut records, mut procs) = (Vec::new(), Vec::new());
    for function in functions {
        // S_GPROC32, whose address field the linker fills in at offset 28.
        procs.push(records.len() + 4 + 28);
        record(
            &mut records,
            0x1110,
            &[&[0; 35][..], function.as_bytes(), &[0]].concat(),
        );
        // S_REGISTER: type 0x74 (int) in register 23 (esi).
        record(&mut records, 0x1106, &[0x74, 0, 0, 0, 23, 0, b'x', 0]);
        record(&mut records, 0x0006, &[]);
    }
    let mut data = 4_u32.to_le_bytes().to_vec();
    let base = subsection(&mut data, 0xf1, &records);
    let mut relocations: Vec<_> = functions
        .iter()
        .zip(procs)
        .map(|(function, at)| {
            (
                (base + at) as u64,
                *function,
                RelocationFlags::Coff {
                    typ: object::pe::IMAGE_REL_I386_SECREL,
                },
            )
        })
        .collect();
    subsection(&mut data, 0xf3, b"\0$T0 $esi =\0");
    for function in functions {
        // The function's address, then one frame entry whose program is string 1.
        let mut frame = vec![0; 36];
        frame[24] = 1;
        let at = subsection(&mut data, 0xf5, &frame);
        relocations.push((
            at as u64,
            *function,
            RelocationFlags::Coff {
                typ: object::pe::IMAGE_REL_I386_DIR32NB,
            },
        ));
    }
    DebugSection {
        name: ".debug$S",
        data,
        relocations,
    }
}

/// Each function's register variable and frame program, read from the
/// `.debug$S` of a C13 object.
fn codeview_registers(object: &[u8]) -> Vec<(u16, String)> {
    let file = object::File::parse(object).expect("object");
    let data = file
        .section_by_name(".debug$S")
        .expect("debug section")
        .data()
        .expect("data");
    let word = |at: usize| u32::from_le_bytes(data[at..at + 4].try_into().expect("word")) as usize;
    let (mut registers, mut programs, mut strings) = (Vec::new(), Vec::new(), 0);
    let mut at = 4;
    while at < data.len() {
        let (kind, length, body) = (word(at), word(at + 4), at + 8);
        match kind {
            0xf1 => {
                let mut record = body;
                while record < body + length {
                    if data[record + 2..record + 4] == [0x06, 0x11] {
                        registers.push(u16::from_le_bytes([data[record + 8], data[record + 9]]));
                    }
                    record += 2 + usize::from(u16::from_le_bytes([data[record], data[record + 1]]));
                }
            }
            0xf3 => strings = body,
            0xf5 => programs.push(word(body + 24)),
            _ => {}
        }
        at = body + length.next_multiple_of(4);
    }
    let text = |offset: usize| {
        let start = strings + offset;
        let end = start
            + data[start..]
                .iter()
                .position(|&byte| byte == 0)
                .expect("terminated");
        String::from_utf8_lossy(&data[start..end]).into_owned()
    };
    registers
        .into_iter()
        .zip(programs.into_iter().map(text))
        .collect()
}

#[test]
fn a_register_rename_carries_into_codeview_records_without_touching_shared_ones() {
    let functions = [
        ("_f", SAVES_ESI_FIRST),
        ("_g", SAVES_ESI_FIRST),
        ("_h", SAVES_ESI_FIRST),
    ];
    let object = i386_object(
        BinaryFormat::Coff,
        &functions,
        vec![codeview(&["_f", "_g", "_h"])],
    );
    let world = object_world("main.obj", &object, &object);
    let project = fs::read_to_string(world.project().join("reprobit.toml")).expect("project");
    let swap = r#"{ esi = "edi", edi = "esi" }"#;
    world.edit(
        "reprobit.toml",
        &project.replace(
            "outputs = [\"main.obj\"]",
            &format!("outputs = [\"main.obj\"]\nregisters = {{ _f = {swap}, _h = {swap} }}"),
        ),
    );
    succeeds(&world.rbit(&["setup"]));

    succeeds(&world.rbit(&["build"]));

    let built = fs::read(world.project().join(".reprobit/stage/build/main.obj")).expect("built");
    let renamed = (24, "$T0 $edi =".to_owned());
    assert_eq!(
        codeview_registers(&built),
        [renamed.clone(), (23, "$T0 $esi =".to_owned()), renamed]
    );
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
