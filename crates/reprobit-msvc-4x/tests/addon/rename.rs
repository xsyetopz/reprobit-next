//! The `rename` rewrite and its proposals.

use std::path::Path;

use super::{addon, entered, function, object};

/// `mov edx, [esp+4]`
const LOAD_EDX: [u8; 4] = [0x8b, 0x54, 0x24, 0x04];
/// `mov ecx, [esp+4]`
const LOAD_ECX: [u8; 4] = [0x8b, 0x4c, 0x24, 0x04];
/// `mov ecx, [esp+8]`
const LOAD_ECX_SECOND: [u8; 4] = [0x8b, 0x4c, 0x24, 0x08];
/// `mov edx, [esp+8]`
const LOAD_EDX_SECOND: [u8; 4] = [0x8b, 0x54, 0x24, 0x08];
/// `add edx, edx`
const DOUBLE_EDX: [u8; 2] = [0x03, 0xd2];
/// `add ecx, ecx`
const DOUBLE_ECX: [u8; 2] = [0x03, 0xc9];
/// `mov eax, edx`
const RETURN_EDX: [u8; 2] = [0x8b, 0xc2];
/// `mov eax, ecx`
const RETURN_ECX: [u8; 2] = [0x8b, 0xc1];
/// `mov edx, 0`
const CLEAR_EDX: [u8; 5] = [0xba, 0, 0, 0, 0];
const RET: [u8; 1] = [0xc3];

fn rename(path: &Path, map: &str) -> std::process::Output {
    addon(&[
        "rewrite",
        path.to_str().unwrap(),
        "rename",
        "--symbol",
        "_f",
        "--map",
        map,
    ])
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_value_moves_to_another_register_along_all_its_uses() {
    let dir = tempfile::tempdir().unwrap();
    let path = object(
        dir.path(),
        "f.obj",
        &[&LOAD_EDX, &DOUBLE_EDX, &RETURN_EDX, &CLEAR_EDX, &RET],
        &[],
        None,
    );
    let output = rename(&path, "0:edx:ecx,1:edx:ecx,2:edx:ecx");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        function(&path).0,
        [&LOAD_ECX[..], &DOUBLE_ECX, &RETURN_ECX, &CLEAR_EDX, &RET].concat()
    );
}

#[test]
fn a_use_renamed_apart_from_its_definition_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = object(
        dir.path(),
        "f.obj",
        &[&LOAD_EDX, &DOUBLE_EDX, &RETURN_EDX, &CLEAR_EDX, &RET],
        &[],
        None,
    );
    let before = std::fs::read(&path).unwrap();
    let output = rename(&path, "1:edx:ecx");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("would read ECX where the original read EDX"),
        "{}",
        stderr(&output)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_place_entered_from_outside_starts_with_the_original_values() {
    let dir = tempfile::tempdir().unwrap();
    // Data names `add edx, edx`, so it may run with any `edx` and `ecx`.
    let path = entered(
        dir.path(),
        "f.obj",
        &[&LOAD_EDX, &DOUBLE_EDX, &RETURN_EDX, &CLEAR_EDX, &RET],
        &[],
        None,
        Some(4),
    );
    let output = rename(&path, "0:edx:ecx,1:edx:ecx,2:edx:ecx");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("would read ECX where the original read EDX"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_register_the_caller_sees_must_keep_its_value() {
    let dir = tempfile::tempdir().unwrap();
    // Without `mov edx, 0`, the caller would see the doubled value in `edx`.
    let path = object(
        dir.path(),
        "f.obj",
        &[&LOAD_EDX, &DOUBLE_EDX, &RETURN_EDX, &RET],
        &[],
        None,
    );
    let output = rename(&path, "0:edx:ecx,1:edx:ecx,2:edx:ecx");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("EDX would hold another value"),
        "{}",
        stderr(&output)
    );
}

/// `mov ecx, 1` / `test eax, eax` / `je +5` / `mov ecx, 2` / `mov eax, ecx`,
/// then `mov edx, 0` and `ret`, with each `ecx` as `edx` when `renamed`.
fn branching(renamed: [bool; 3]) -> Vec<Vec<u8>> {
    let pick = |rename: bool, ecx: Vec<u8>, edx: Vec<u8>| if rename { edx } else { ecx };
    vec![
        pick(renamed[0], vec![0xb9, 1, 0, 0, 0], vec![0xba, 1, 0, 0, 0]),
        vec![0x85, 0xc0],
        vec![0x74, 0x05],
        pick(renamed[1], vec![0xb9, 2, 0, 0, 0], vec![0xba, 2, 0, 0, 0]),
        pick(renamed[2], vec![0x8b, 0xc1], vec![0x8b, 0xc2]),
        CLEAR_EDX.to_vec(),
        RET.to_vec(),
    ]
}

#[test]
fn a_value_that_merges_from_two_paths_moves_only_when_both_move() {
    let dir = tempfile::tempdir().unwrap();
    let original = branching([false; 3]);
    let parts: Vec<&[u8]> = original.iter().map(Vec::as_slice).collect();
    let one_path = object(dir.path(), "one.obj", &parts, &[], None);
    assert!(
        !rename(&one_path, "0:ecx:edx,4:ecx:edx").status.success(),
        "the path through `mov ecx, 2` still writes ecx"
    );
    assert!(
        !rename(&one_path, "3:ecx:edx,4:ecx:edx").status.success(),
        "the path that skips `mov ecx, 2` still carries ecx"
    );
    let both = object(dir.path(), "both.obj", &parts, &[], None);
    let output = rename(&both, "0:ecx:edx,3:ecx:edx,4:ecx:edx");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(function(&both).0, branching([true; 3]).concat());
}

#[test]
fn propose_finds_the_registers_that_trade_places() {
    let dir = tempfile::tempdir().unwrap();
    // `sub ecx, edx` and `sub edx, ecx`, then `mov eax, ecx` and `mov eax, edx`.
    let built = object(
        dir.path(),
        "built.obj",
        &[
            &LOAD_ECX,
            &LOAD_EDX_SECOND,
            &[0x2b, 0xca],
            &RETURN_ECX,
            &CLEAR_EDX,
            &RET,
        ],
        &[],
        None,
    );
    let reference = object(
        dir.path(),
        "reference.obj",
        &[
            &LOAD_EDX,
            &LOAD_ECX_SECOND,
            &[0x2b, 0xd1],
            &RETURN_EDX,
            &CLEAR_EDX,
            &RET,
        ],
        &[],
        None,
    );
    let output = addon(&[
        "propose",
        built.to_str().unwrap(),
        reference.to_str().unwrap(),
        "_f",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let proposals: Vec<Vec<String>> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        proposals,
        vec![vec![
            "rename",
            "--symbol",
            "_f",
            "--map",
            "0:ecx:edx,1:edx:ecx,2:ecx:edx,2:edx:ecx,3:ecx:edx"
        ]]
    );
    let mut args = vec!["rewrite", built.to_str().unwrap()];
    args.extend(proposals[0].iter().map(String::as_str));
    assert!(addon(&args).status.success());
    assert_eq!(function(&built), function(&reference));
}
