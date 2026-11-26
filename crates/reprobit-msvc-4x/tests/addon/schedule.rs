//! The `schedule` rewrite and its proposals.

use std::path::Path;

use super::{addon, function, object};

/// `mov eax, [_g]`, relocated at +1.
const LOAD: [u8; 5] = [0xa1, 0, 0, 0, 0];
/// `mov ecx, 5`
const SET_ECX: [u8; 5] = [0xb9, 5, 0, 0, 0];
/// `mov edx, 1`
const SET_EDX: [u8; 5] = [0xba, 1, 0, 0, 0];
/// `mov ebx, 2`
const SET_EBX: [u8; 5] = [0xbb, 2, 0, 0, 0];
const RET: [u8; 1] = [0xc3];

fn schedule(path: &Path, order: &str) -> std::process::Output {
    addon(&[
        "rewrite",
        path.to_str().unwrap(),
        "schedule",
        "--symbol",
        "_f",
        "--order",
        order,
    ])
}

#[test]
fn an_independent_load_moves_and_its_relocation_moves_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = object(dir.path(), "f.obj", &[&LOAD, &SET_ECX, &RET], &[1], None);
    let output = schedule(&path, "1,0,2");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        function(&path),
        ([&SET_ECX[..], &LOAD, &RET].concat(), vec![6])
    );
}

#[test]
fn an_order_that_breaks_a_dependence_is_refused_and_the_object_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    // `add ecx, 1` reads what `mov ecx, 5` wrote.
    let add_ecx: [u8; 3] = [0x83, 0xc1, 1];
    let path = object(dir.path(), "f.obj", &[&SET_ECX, &add_ecx, &RET], &[], None);
    let before = std::fs::read(&path).unwrap();
    let output = schedule(&path, "1,0,2");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("depend on each other"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(
        !schedule(&path, "0,1").status.success(),
        "an order must list every instruction"
    );
}

#[test]
fn the_end_of_an_fpo_prologue_keeps_its_place() {
    let dir = tempfile::tempdir().unwrap();
    let code: [&[u8]; 5] = [&SET_ECX, &SET_EDX, &LOAD, &SET_EBX, &RET];
    let free = object(dir.path(), "free.obj", &code, &[11], None);
    assert!(
        schedule(&free, "2,0,1,3,4").status.success(),
        "without FPO data the load may move first"
    );
    let fpo = object(dir.path(), "fpo.obj", &code, &[11], Some(5));
    assert!(
        !schedule(&fpo, "2,0,1,3,4").status.success(),
        "the load may not cross the prologue's end"
    );
    assert!(
        schedule(&fpo, "0,1,3,2,4").status.success(),
        "instructions after the prologue may still trade places"
    );
}

#[test]
fn propose_finds_the_order_that_rebuilds_the_reference() {
    let dir = tempfile::tempdir().unwrap();
    let built = object(
        dir.path(),
        "built.obj",
        &[&LOAD, &SET_ECX, &SET_EDX, &RET],
        &[1],
        None,
    );
    let reference = object(
        dir.path(),
        "reference.obj",
        &[&SET_EDX, &LOAD, &SET_ECX, &RET],
        &[6],
        None,
    );
    let output = addon(&[
        "propose",
        built.to_str().unwrap(),
        reference.to_str().unwrap(),
        "_f",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let proposals: Vec<Vec<String>> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        proposals,
        vec![vec!["schedule", "--symbol", "_f", "--order", "2,0,1,3"]]
    );
    let mut args = vec!["rewrite", built.to_str().unwrap()];
    args.extend(proposals[0].iter().map(String::as_str));
    assert!(addon(&args).status.success());
    assert_eq!(function(&built), function(&reference));
    let same = addon(&[
        "propose",
        built.to_str().unwrap(),
        reference.to_str().unwrap(),
        "_f",
    ]);
    assert_eq!(
        String::from_utf8_lossy(&same.stdout).trim(),
        "[]",
        "a matching function needs no rewrite"
    );
}

/// `push edx`
const PUSH_EDX: [u8; 1] = [0x52];
/// `mov eax, [esp+0x18]`
const LOAD_ARGUMENT: [u8; 4] = [0x8b, 0x44, 0x24, 0x18];
/// `mov eax, [esp+0x1c]`, the same slot one push deeper.
const LOAD_ARGUMENT_PUSHED: [u8; 4] = [0x8b, 0x44, 0x24, 0x1c];

#[test]
fn a_stack_load_crosses_a_push_and_its_displacement_follows_the_slot() {
    let dir = tempfile::tempdir().unwrap();
    let path = object(
        dir.path(),
        "f.obj",
        &[&LOAD_ARGUMENT, &PUSH_EDX, &RET],
        &[],
        None,
    );
    let output = schedule(&path, "1,0,2");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        function(&path).0,
        [&PUSH_EDX[..], &LOAD_ARGUMENT_PUSHED, &RET].concat()
    );
}

#[test]
fn a_stack_load_of_the_pushed_slot_keeps_its_place() {
    let dir = tempfile::tempdir().unwrap();
    // `mov eax, [esp]` after `push edx` reads the value just pushed.
    let load_top: [u8; 3] = [0x8b, 0x04, 0x24];
    let path = object(
        dir.path(),
        "f.obj",
        &[&PUSH_EDX, &load_top, &RET],
        &[],
        None,
    );
    let output = schedule(&path, "1,0,2");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("depend on each other"));
}

#[test]
fn a_rebased_displacement_that_outgrows_its_encoding_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    // `mov eax, [esp+0x7c]` needs 0x80 below a push, which a disp8 cannot hold.
    let load_far: [u8; 4] = [0x8b, 0x44, 0x24, 0x7c];
    let path = object(
        dir.path(),
        "f.obj",
        &[&load_far, &PUSH_EDX, &RET],
        &[],
        None,
    );
    let before = std::fs::read(&path).unwrap();
    let output = schedule(&path, "1,0,2");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot hold"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_load_through_another_register_does_not_cross_a_push() {
    let dir = tempfile::tempdir().unwrap();
    // `mov eax, [ecx]` may point at the stack.
    let load_pointer: [u8; 2] = [0x8b, 0x01];
    let path = object(
        dir.path(),
        "f.obj",
        &[&load_pointer, &PUSH_EDX, &RET],
        &[],
        None,
    );
    assert!(!schedule(&path, "1,0,2").status.success());
}

#[test]
fn propose_matches_a_stack_load_by_its_slot() {
    let dir = tempfile::tempdir().unwrap();
    let built = object(
        dir.path(),
        "built.obj",
        &[&LOAD_ARGUMENT, &PUSH_EDX, &RET],
        &[],
        None,
    );
    let reference = object(
        dir.path(),
        "reference.obj",
        &[&PUSH_EDX, &LOAD_ARGUMENT_PUSHED, &RET],
        &[],
        None,
    );
    let output = addon(&[
        "propose",
        built.to_str().unwrap(),
        reference.to_str().unwrap(),
        "_f",
    ]);
    let proposals: Vec<Vec<String>> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        proposals,
        vec![vec!["schedule", "--symbol", "_f", "--order", "1,0,2"]]
    );
}
