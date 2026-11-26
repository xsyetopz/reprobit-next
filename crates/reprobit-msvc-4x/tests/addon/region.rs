//! The `region` rewrite and its proposals.

use std::path::Path;

use super::{addon, function, object};

/// `mov eax, [esp+4]`
const FIRST_EAX: [u8; 4] = [0x8b, 0x44, 0x24, 0x04];
/// `mov ecx, [esp+4]`
const FIRST_ECX: [u8; 4] = [0x8b, 0x4c, 0x24, 0x04];
/// `mov eax, [esp+8]`
const SECOND_EAX: [u8; 4] = [0x8b, 0x44, 0x24, 0x08];
/// `mov ecx, [esp+8]`
const SECOND_ECX: [u8; 4] = [0x8b, 0x4c, 0x24, 0x08];
/// `add eax, ecx`
const ADD: [u8; 2] = [0x03, 0xc1];
/// `imul eax, ecx`
const IMUL: [u8; 3] = [0x0f, 0xaf, 0xc1];
/// `sub eax, ecx`
const SUB: [u8; 2] = [0x2b, 0xc1];
const RET: [u8; 1] = [0xc3];

fn region(path: &Path, rewrite: &str) -> std::process::Output {
    addon(&[
        "rewrite",
        path.to_str().unwrap(),
        "region",
        "--symbol",
        "_f",
        "--rewrite",
        rewrite,
    ])
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_product_keeps_its_value_when_its_operands_trade_registers() {
    let dir = tempfile::tempdir().unwrap();
    let path = object(
        dir.path(),
        "f.obj",
        &[&FIRST_EAX, &SECOND_ECX, &IMUL, &RET],
        &[],
        None,
    );
    let output = region(&path, "0,1,2/0:eax:ecx,1:ecx:eax");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        function(&path).0,
        [&FIRST_ECX[..], &SECOND_EAX, &IMUL, &RET].concat()
    );
}

#[test]
fn a_difference_whose_operands_trade_registers_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = object(
        dir.path(),
        "f.obj",
        &[&FIRST_EAX, &SECOND_ECX, &SUB, &RET],
        &[],
        None,
    );
    let before = std::fs::read(&path).unwrap();
    let output = region(&path, "0,1,2/0:eax:ecx,1:ecx:eax");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("EAX would hold another value"),
        "{}",
        stderr(&output)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn stores_that_may_overlap_keep_their_order() {
    let dir = tempfile::tempdir().unwrap();
    // `mov [ebx], eax` and `mov [ecx], edx`
    let path = object(
        dir.path(),
        "f.obj",
        &[&[0x89, 0x03], &[0x89, 0x11], &RET],
        &[],
        None,
    );
    let output = region(&path, "1,0");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("leave memory different"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_run_that_a_branch_enters_in_its_middle_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    // `test eax, eax` and `je` to `mov ecx, [esp+8]`
    let path = object(
        dir.path(),
        "f.obj",
        &[
            &[0x85, 0xc0],
            &[0x74, 0x04],
            &FIRST_EAX,
            &SECOND_ECX,
            &ADD,
            &RET,
        ],
        &[],
        None,
    );
    let output = region(&path, "2,3,4/2:eax:ecx,3:ecx:eax");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("control reaches 0x8"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn propose_reorders_and_renames_a_run_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let built = object(
        dir.path(),
        "built.obj",
        &[&FIRST_EAX, &SECOND_ECX, &ADD, &RET],
        &[],
        None,
    );
    let reference = object(
        dir.path(),
        "reference.obj",
        &[&SECOND_EAX, &FIRST_ECX, &ADD, &RET],
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
            "region",
            "--symbol",
            "_f",
            "--rewrite",
            "1,0,2/0:eax:ecx,1:ecx:eax"
        ]]
    );
    let mut args = vec!["rewrite", built.to_str().unwrap()];
    args.extend(proposals[0].iter().map(String::as_str));
    assert!(addon(&args).status.success());
    assert_eq!(function(&built), function(&reference));
}

#[test]
fn stores_to_separate_stack_slots_trade_places() {
    let dir = tempfile::tempdir().unwrap();
    // `mov [esp+4], eax` and `mov [esp+8], ecx`
    let (first, second): (&[u8], &[u8]) = (&[0x89, 0x44, 0x24, 0x04], &[0x89, 0x4c, 0x24, 0x08]);
    let path = object(dir.path(), "f.obj", &[first, second, &RET], &[], None);
    let output = region(&path, "1,0");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(function(&path).0, [second, first, &RET].concat());
}

#[test]
fn a_sum_keeps_its_value_when_its_addends_come_in_another_order() {
    let dir = tempfile::tempdir().unwrap();
    // `add eax, [esp+8]` and `add eax, [esp+12]`, whose flags differ between
    // the orders but are dead at `ret`.
    let (second, third): (&[u8], &[u8]) = (&[0x03, 0x44, 0x24, 0x08], &[0x03, 0x44, 0x24, 0x0c]);
    let path = object(
        dir.path(),
        "f.obj",
        &[&FIRST_EAX, second, third, &RET],
        &[],
        None,
    );
    let output = region(&path, "0,2,1");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        function(&path).0,
        [&FIRST_EAX[..], third, second, &RET].concat()
    );
}

#[test]
fn x87_loads_keep_their_order() {
    let dir = tempfile::tempdir().unwrap();
    // `fld dword [esp+4]`, `fld dword [esp+8]`, and `fsubp`, whose difference
    // turns around when the loads trade places.
    let path = object(
        dir.path(),
        "f.obj",
        &[
            &[0xd9, 0x44, 0x24, 0x04],
            &[0xd9, 0x44, 0x24, 0x08],
            &[0xde, 0xe9],
            &RET,
        ],
        &[],
        None,
    );
    let output = region(&path, "1,0,2");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("x87 state different"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_register_zeroed_for_a_sum_may_be_another() {
    let dir = tempfile::tempdir().unwrap();
    // `xor ecx, ecx` / `add eax, ecx`, then `mov edx, 0` before `ret`, so
    // `edx` may hold the zero in the run.
    let clear = [0xba, 0, 0, 0, 0];
    let path = object(
        dir.path(),
        "f.obj",
        &[&FIRST_EAX, &[0x33, 0xc9], &ADD, &clear, &RET],
        &[],
        None,
    );
    let output = region(&path, "0,1,2/1:ecx:edx,2:ecx:edx");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        function(&path).0,
        [&FIRST_EAX[..], &[0x33, 0xd2], &[0x03, 0xc2], &clear, &RET].concat()
    );
}
