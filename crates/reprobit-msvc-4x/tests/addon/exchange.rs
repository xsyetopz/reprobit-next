//! The `exchange` rewrite and its proposals.

use std::path::Path;

use super::{addon, function, object};

const LOAD_EAX: [u8; 4] = [0x8b, 0x44, 0x24, 0x04];
const LOAD_ECX: [u8; 4] = [0x8b, 0x4c, 0x24, 0x08];
/// `cmp eax, ecx`
const COMPARE: [u8; 2] = [0x3b, 0xc1];
/// `jl +2`, over `mov eax, ecx`.
const LESS: [u8; 2] = [0x7c, 0x02];
const MOVE: [u8; 2] = [0x8b, 0xc1];
const RET: [u8; 1] = [0xc3];

fn exchange(path: &Path, sites: &str) -> std::process::Output {
    addon(&[
        "rewrite",
        path.to_str().unwrap(),
        "exchange",
        "--symbol",
        "_f",
        "--at",
        sites,
    ])
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn minimum(dir: &Path, name: &str) -> std::path::PathBuf {
    object(
        dir,
        name,
        &[&LOAD_EAX, &LOAD_ECX, &COMPARE, &LESS, &MOVE, &RET],
        &[],
        None,
    )
}

#[test]
fn a_compare_turns_around_with_its_branch_mirrored() {
    let dir = tempfile::tempdir().unwrap();
    let path = minimum(dir.path(), "f.obj");
    let output = exchange(&path, "2");
    assert!(output.status.success(), "{}", stderr(&output));
    // `cmp ecx, eax` by its other opcode, and `jg`.
    assert_eq!(
        function(&path).0,
        [
            &LOAD_EAX[..],
            &LOAD_ECX,
            &[0x39, 0xc1],
            &[0x7f, 0x02],
            &MOVE,
            &RET
        ]
        .concat()
    );
    let fields = minimum(dir.path(), "fields.obj");
    let output = exchange(&fields, "2:fields");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        function(&fields).0,
        [
            &LOAD_EAX[..],
            &LOAD_ECX,
            &[0x3b, 0xc8],
            &[0x7f, 0x02],
            &MOVE,
            &RET
        ]
        .concat()
    );
}

#[test]
fn a_compare_keeps_its_order_when_a_later_instruction_reads_its_carry() {
    let dir = tempfile::tempdir().unwrap();
    // `jl +3` over `adc eax, 0`, which reads the carry the compare set.
    let path = object(
        dir.path(),
        "f.obj",
        &[
            &LOAD_EAX,
            &LOAD_ECX,
            &COMPARE,
            &[0x7c, 0x03],
            &[0x83, 0xd0, 0x00],
            &RET,
        ],
        &[],
        None,
    );
    let before = std::fs::read(&path).unwrap();
    let output = exchange(&path, "2");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("read CF"), "{}", stderr(&output));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_branch_on_the_sign_has_no_mirror() {
    let dir = tempfile::tempdir().unwrap();
    let path = object(
        dir.path(),
        "f.obj",
        &[&LOAD_EAX, &LOAD_ECX, &COMPARE, &[0x78, 0x02], &MOVE, &RET],
        &[],
        None,
    );
    let output = exchange(&path, "2");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("mirror"), "{}", stderr(&output));
}

#[test]
fn an_x87_load_and_product_trade_operands() {
    let dir = tempfile::tempdir().unwrap();
    // `fld dword [esp+4]`, `fmul dword [esp+8]`.
    let path = object(
        dir.path(),
        "f.obj",
        &[&[0xd9, 0x44, 0x24, 0x04], &[0xd8, 0x4c, 0x24, 0x08], &RET],
        &[],
        None,
    );
    let output = exchange(&path, "0");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        function(&path).0,
        [0xd9, 0x44, 0x24, 0x08, 0xd8, 0x4c, 0x24, 0x04, 0xc3]
    );
}

#[test]
fn an_x87_pair_keeps_its_operands_when_a_branch_enters_between_them() {
    let dir = tempfile::tempdir().unwrap();
    // `test eax, eax`, `je +4` to `fadd dword [eax]` after `fld dword [esp+4]`,
    // whose start moves when the operands, of different lengths, trade places.
    let path = object(
        dir.path(),
        "f.obj",
        &[
            &[0x85, 0xc0],
            &[0x74, 0x04],
            &[0xd9, 0x44, 0x24, 0x04],
            &[0xd8, 0x00],
            &RET,
        ],
        &[],
        None,
    );
    let output = exchange(&path, "2");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("control reaches"),
        "{}",
        stderr(&output)
    );
}

/// `mov eax, [ebp+a]; cmp [ebp+b], eax; jne` to `mov eax, 2`, else `mov eax, 1`,
/// then `ret`; with `loaded`, the not-equal path returns `eax` as loaded.
fn equality(a: u8, b: u8, loaded: bool) -> Vec<Vec<u8>> {
    let mut code = vec![vec![0x8b, 0x45, a], vec![0x39, 0x45, b]];
    if loaded {
        code.extend([vec![0x75, 0x05], vec![0xb8, 1, 0, 0, 0], vec![0xc3]]);
    } else {
        code.extend([
            vec![0x75, 0x07],
            vec![0xb8, 1, 0, 0, 0],
            vec![0xeb, 0x05],
            vec![0xb8, 2, 0, 0, 0],
            vec![0xc3],
        ]);
    }
    code
}

fn parts(code: &[Vec<u8>]) -> Vec<&[u8]> {
    code.iter().map(Vec::as_slice).collect()
}

#[test]
fn an_equality_test_loads_either_slot() {
    let dir = tempfile::tempdir().unwrap();
    let path = object(
        dir.path(),
        "f.obj",
        &parts(&equality(8, 12, false)),
        &[],
        None,
    );
    let output = exchange(&path, "0");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(function(&path).0, equality(12, 8, false).concat());
}

#[test]
fn an_equality_test_keeps_its_slots_when_the_loaded_value_is_returned() {
    let dir = tempfile::tempdir().unwrap();
    let path = object(
        dir.path(),
        "f.obj",
        &parts(&equality(8, 12, true)),
        &[],
        None,
    );
    let output = exchange(&path, "0");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("EAX would hold another value"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn propose_finds_the_mirrored_compare() {
    let dir = tempfile::tempdir().unwrap();
    let built = minimum(dir.path(), "built.obj");
    let reference = object(
        dir.path(),
        "reference.obj",
        &[
            &LOAD_EAX,
            &LOAD_ECX,
            &[0x3b, 0xc8],
            &[0x7f, 0x02],
            &MOVE,
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
        vec![vec!["exchange", "--symbol", "_f", "--at", "2:fields"]]
    );
    let mut args = vec!["rewrite", built.to_str().unwrap()];
    args.extend(proposals[0].iter().map(String::as_str));
    assert!(addon(&args).status.success());
    assert_eq!(function(&built), function(&reference));
}

#[test]
fn propose_chains_a_mirror_with_a_rename() {
    let dir = tempfile::tempdir().unwrap();
    let clear_edx: &[u8] = &[0xba, 0, 0, 0, 0];
    let built = object(
        dir.path(),
        "built.obj",
        &[
            &LOAD_EAX, &LOAD_ECX, &COMPARE, &LESS, &MOVE, clear_edx, &RET,
        ],
        &[],
        None,
    );
    // `mov edx, [esp+8]`, `cmp edx, eax`, `jg`, `mov eax, edx`.
    let reference = object(
        dir.path(),
        "reference.obj",
        &[
            &LOAD_EAX,
            &[0x8b, 0x54, 0x24, 0x08],
            &[0x39, 0xc2],
            &[0x7f, 0x02],
            &[0x8b, 0xc2],
            clear_edx,
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
            "exchange",
            "--symbol",
            "_f",
            "--at",
            "2",
            "rename",
            "--symbol",
            "_f",
            "--map",
            "1:ecx:edx,2:ecx:edx,4:ecx:edx"
        ]]
    );
    let mut args = vec!["rewrite", built.to_str().unwrap()];
    args.extend(proposals[0].iter().map(String::as_str));
    assert!(addon(&args).status.success());
    assert_eq!(function(&built), function(&reference));
}

#[test]
fn a_chain_writes_nothing_when_a_later_rewrite_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = minimum(dir.path(), "f.obj");
    let before = std::fs::read(&path).unwrap();
    let output = addon(&[
        "rewrite",
        path.to_str().unwrap(),
        "exchange",
        "--symbol",
        "_f",
        "--at",
        "2",
        "rename",
        "--symbol",
        "_f",
        "--map",
        "4:ecx:edx",
    ]);
    assert!(!output.status.success());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
