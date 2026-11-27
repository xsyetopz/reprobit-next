//! Per-function grades: exact, reloc, different, and missing, with the
//! progress totals. The objects are built here, so no compiler is needed.

use std::fs;

use object::{
    Architecture, BinaryFormat, Endianness, RelocationFlags, SectionKind, SymbolFlags, SymbolKind,
    SymbolScope, pe, write,
};

use super::{Run, World, succeeds};

/// `mov eax, [addr]; ret`, with `addr` relocated at offset 1.
const LOAD: &[u8] = &[0xa1, 0, 0, 0, 0, 0xc3];
/// `xor eax, eax; ret`
const ZERO: &[u8] = &[0x31, 0xc0, 0xc3];
/// `mov eax, 1; ret`
const ONE: &[u8] = &[0xb8, 1, 0, 0, 0, 0xc3];
/// `mov eax, 2; ret`
const TWO: &[u8] = &[0xb8, 2, 0, 0, 0, 0xc3];

/// A COFF i386 object of `(name, code, symbol its offset 1 loads)` functions.
fn object(functions: &[(&str, &[u8], Option<&str>)]) -> Vec<u8> {
    let mut object = write::Object::new(BinaryFormat::Coff, Architecture::I386, Endianness::Little);
    object.set_mangling(write::Mangling::None);
    let text = object.add_section(Vec::new(), b".text".to_vec(), SectionKind::Text);
    for (name, code, loads) in functions {
        let value = object.append_section_data(text, code, 1);
        object.add_symbol(write::Symbol {
            name: name.as_bytes().to_vec(),
            value,
            size: code.len() as u64,
            kind: SymbolKind::Text,
            scope: SymbolScope::Linkage,
            weak: false,
            section: write::SymbolSection::Section(text),
            flags: SymbolFlags::None,
        });
        if let Some(target) = loads {
            let symbol = object.add_symbol(write::Symbol {
                name: target.as_bytes().to_vec(),
                value: 0,
                size: 0,
                kind: SymbolKind::Data,
                scope: SymbolScope::Linkage,
                weak: false,
                section: write::SymbolSection::Undefined,
                flags: SymbolFlags::None,
            });
            object
                .add_relocation(
                    text,
                    write::Relocation {
                        offset: value + 1,
                        symbol,
                        addend: 0,
                        flags: RelocationFlags::Coff {
                            typ: pe::IMAGE_REL_I386_DIR32,
                        },
                    },
                )
                .expect("relocation");
        }
    }
    object.write().expect("object")
}

fn score(world: &World, reference: &[u8], built: &[u8], extra: &[&str]) -> Run {
    fs::write(world.project().join("reference.obj"), reference).expect("reference");
    fs::write(world.project().join("built.obj"), built).expect("built");
    let mut args = vec!["score", "reference.obj", "built.obj"];
    args.extend(extra);
    world.rbit(&args)
}

fn grades(run: &Run) -> Vec<(String, String)> {
    run.json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .map(|item| {
            (
                item["name"].as_str().unwrap_or_default().to_owned(),
                item["grade"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

#[test]
fn score_grades_each_function_and_counts_progress() {
    let world = World::new();
    let reference = object(&[
        ("_same", LOAD, Some("_global")),
        ("_literal", LOAD, Some("$SG100")),
        ("_changed", ONE, None),
        ("_gone", ZERO, None),
    ]);
    let built = object(&[
        ("_same", LOAD, Some("_global")),
        ("_literal", LOAD, Some("$SG7")),
        ("_changed", TWO, None),
    ]);
    let run = score(&world, &reference, &built, &[]);
    assert_eq!(run.code, 1, "a difference fails the run: {}", run.stderr);
    assert_eq!(
        grades(&run),
        [
            ("_same", "exact"),
            ("_literal", "reloc"),
            ("_changed", "different"),
            ("_gone", "missing"),
        ]
        .map(|(name, grade)| (name.to_owned(), grade.to_owned()))
    );
    assert_eq!(
        (
            &run.json["functions"],
            &run.json["exact"],
            &run.json["reloc"],
            &run.json["different"],
            &run.json["missing"],
            &run.json["matched_percent"],
        ),
        (
            &4.into(),
            &1.into(),
            &1.into(),
            &1.into(),
            &1.into(),
            &50.0.into()
        )
    );
    let changed = &run.json["symbols"][2]["matching_percent"];
    assert!(
        changed
            .as_f64()
            .is_some_and(|value| value > 0.0 && value < 100.0),
        "{changed}"
    );
}

#[test]
fn score_passes_when_the_named_functions_match_and_rejects_unknown_ones() {
    let world = World::new();
    let reference = object(&[("_a", ZERO, None), ("_b", ONE, None)]);
    let built = object(&[("_a", ZERO, None), ("_b", ZERO, None)]);
    let run = score(&world, &reference, &built, &["--symbol", "_a"]);
    assert_eq!(succeeds(&run)["matched_percent"], 100.0);
    assert_eq!(grades(&run), [("_a".to_owned(), "exact".to_owned())]);

    let unknown = score(&world, &reference, &built, &["--symbol", "_nope"]);
    assert_eq!(unknown.code, 2, "{}", unknown.json);
    assert!(
        unknown.json["error"]
            .as_str()
            .is_some_and(|error| error.contains("_nope")),
        "{}",
        unknown.json
    );
}

const UNIT_PROJECT: &str = r#"
name = "units"

[toolchain]
compiler = "fake-cc"

[[step]]
id = "compile.main"
program = "cc"
args = ["${SOURCE}/main.c", "obj/main.obj"]
inputs = ["source/main.c"]
outputs = ["obj/main.obj"]
reference-object = "reference/main.obj"
"#;

/// A project whose one step "compiles" `built` and names `reference` as its original.
fn unit_world(reference: &[u8], built: &[u8]) -> World {
    let world = World::new();
    world.edit("reprobit.toml", UNIT_PROJECT);
    fs::write(world.project().join("main.c"), built).expect("source");
    fs::write(world.project().join("reference/main.obj"), reference).expect("reference");
    world
}

#[test]
fn score_with_no_objects_grades_every_unit_of_the_project() {
    let reference = object(&[("_kept", ZERO, None), ("_changed", ONE, None)]);
    let world = unit_world(
        &reference,
        &object(&[("_kept", ZERO, None), ("_changed", TWO, None)]),
    );

    let unbuilt = world.rbit(&["score"]);
    assert_eq!(unbuilt.code, 1, "{}", unbuilt.stderr);
    assert_eq!(
        (&unbuilt.json["functions"], &unbuilt.json["missing"]),
        (&2.into(), &2.into())
    );

    succeeds(&world.rbit(&["setup"]));
    succeeds(&world.rbit(&["build"]));
    let built = world.rbit(&["score"]);
    assert_eq!(built.code, 1, "{}", built.stderr);
    assert_eq!(
        grades(&built),
        [("_kept", "exact"), ("_changed", "different")]
            .map(|(name, grade)| (name.to_owned(), grade.to_owned()))
    );
    assert_eq!(built.json["symbols"][0]["unit"], "compile.main");
    assert_eq!(built.json["matched_percent"], 50.0);

    let one = succeeds(&world.rbit(&["score", "--symbol", "_kept"])).clone();
    assert_eq!((&one["functions"], &one["exact"]), (&1.into(), &1.into()));
}

/// A built project whose `_kept` matches and `_changed` differs, scored once so that
/// `_kept` is recorded; then `_kept` is changed and rebuilt.
fn regressed_world() -> World {
    let reference = object(&[("_kept", ZERO, None), ("_changed", ONE, None)]);
    let world = unit_world(
        &reference,
        &object(&[("_kept", ZERO, None), ("_changed", TWO, None)]),
    );
    succeeds(&world.rbit(&["setup"]));
    succeeds(&world.rbit(&["build"]));
    let first = world.rbit(&["score"]);
    assert_eq!(first.json["regressed"], 0, "{}", first.json);
    fs::write(
        world.project().join("main.c"),
        object(&[("_kept", ONE, None), ("_changed", TWO, None)]),
    )
    .expect("source");
    succeeds(&world.rbit(&["build"]));
    world
}

fn regressed(run: &Run) -> Vec<String> {
    run.json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .filter(|item| item["regressed"] == true)
        .filter_map(|item| item["name"].as_str().map(str::to_owned))
        .collect()
}

/// SCORE-1: a function that matched in an earlier score and no longer does is regressed.
#[test]
fn score_names_a_function_that_matched_before_as_regressed() {
    let world = regressed_world();
    let run = world.rbit(&["score"]);
    assert_eq!(run.code, 1, "{}", run.stderr);
    assert_eq!(run.json["regressed"], 1, "{}", run.json);
    assert_eq!(regressed(&run), ["_kept"]);
}

/// SCORE-2: `--regressions` passes on unfinished work and fails on a regression.
#[test]
fn score_regressions_fails_only_when_a_match_was_lost() {
    let reference = object(&[("_kept", ZERO, None), ("_changed", ONE, None)]);
    let world = unit_world(
        &reference,
        &object(&[("_kept", ZERO, None), ("_changed", TWO, None)]),
    );
    succeeds(&world.rbit(&["setup"]));
    succeeds(&world.rbit(&["build"]));
    assert_eq!(world.rbit(&["score"]).code, 1);
    let unfinished = world.rbit(&["score", "--regressions"]);
    assert_eq!(unfinished.code, 0, "{}", unfinished.json);

    let world = regressed_world();
    let lost = world.rbit(&["score", "--regressions"]);
    assert_eq!(lost.code, 1, "{}", lost.json);
    assert_eq!(regressed(&lost), ["_kept"]);
}

/// SCORE-3: `--forget` drops the regressed function, so the next score reports none.
#[test]
fn score_forget_drops_a_regressed_function_from_the_record() {
    let world = regressed_world();
    world.rbit(&["score", "--forget"]);
    let after = world.rbit(&["score", "--regressions"]);
    assert_eq!(after.code, 0, "{}", after.json);
    assert_eq!(after.json["regressed"], 0);
    assert_eq!(regressed(&after), Vec::<String>::new());
}

#[test]
fn objdiff_config_points_at_each_unit_and_its_build_command_works() {
    let reference = object(&[("_f", ZERO, None)]);
    let world = unit_world(&reference, &reference);
    succeeds(&world.rbit(&["setup"]));
    succeeds(&world.rbit(&["objdiff"]));
    let config: serde_json::Value = serde_json::from_slice(
        &fs::read(world.project().join("objdiff.json")).expect("objdiff.json"),
    )
    .expect("objdiff.json is JSON");
    let unit = &config["units"][0];
    assert_eq!(
        (
            &unit["name"],
            &unit["target_path"],
            &unit["metadata"]["source_path"]
        ),
        (
            &"compile.main".into(),
            &"reference/main.obj".into(),
            &"main.c".into()
        )
    );
    let base = unit["base_path"].as_str().expect("base_path").to_owned();

    // objdiff runs `custom_make custom_args… base_path` in the project.
    let mut args: Vec<String> = config["custom_args"]
        .as_array()
        .expect("custom_args")
        .iter()
        .filter_map(|arg| arg.as_str().map(str::to_owned))
        .collect();
    assert_eq!(config["custom_make"], "rbit");
    args.push(base.clone());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    succeeds(&world.rbit(&refs));
    assert_eq!(
        fs::read(world.project().join(&base)).expect("built object"),
        reference
    );
    assert_eq!(
        world
            .rbit(&["build", "--object", "elsewhere/main.obj"])
            .code,
        2
    );
}
