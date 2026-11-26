//! The `rename` rewrite and its proposals.

use std::path::Path;

use object::write::{Mangling, Object, Relocation, StandardSection, Symbol, SymbolSection};
use object::{
    Architecture, BinaryFormat, Endianness, Object as _, ObjectSection as _, ObjectSymbol as _,
    RelocationFlags, SymbolFlags, SymbolKind, SymbolScope,
};

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
/// `int3`, the padding after a function.
const INT3: [u8; 1] = [0xcc];
/// `mov ebx, [esp+4]`
const LOAD_EBX: [u8; 4] = [0x8b, 0x5c, 0x24, 0x04];
/// `mov ebp, [esp+4]`
const LOAD_EBP: [u8; 4] = [0x8b, 0x6c, 0x24, 0x04];
/// `mov eax, [ebx]`
const READ_EBX: [u8; 2] = [0x8b, 0x03];
/// `mov eax, [ebp+0]`, one byte longer, as `ebp` needs a displacement.
const READ_EBP: [u8; 3] = [0x8b, 0x45, 0x00];
/// `xor ebx, ebx` / `xor ebp, ebp`, so the caller sees both registers alike.
const CLEAR_BOTH: [u8; 4] = [0x33, 0xdb, 0x33, 0xed];

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

/// `mov ebx, [esp+4]` / `xor eax, eax` / `test ebx, ebx` / `je` over
/// `mov eax, [ebx]` / `xor ebx, ebx` / `xor ebp, ebp` / `ret` / `int3`, or with `ebp` and no padding.
fn guarded(ebp: bool) -> Vec<Vec<u8>> {
    if ebp {
        vec![
            LOAD_EBP.to_vec(),
            vec![0x33, 0xc0],
            vec![0x85, 0xed],
            vec![0x74, 0x03],
            READ_EBP.to_vec(),
            CLEAR_BOTH.to_vec(),
            RET.to_vec(),
        ]
    } else {
        vec![
            LOAD_EBX.to_vec(),
            vec![0x33, 0xc0],
            vec![0x85, 0xdb],
            vec![0x74, 0x02],
            READ_EBX.to_vec(),
            CLEAR_BOTH.to_vec(),
            RET.to_vec(),
            INT3.to_vec(),
        ]
    }
}

#[test]
fn a_longer_base_takes_the_padding_and_its_branch_follows() {
    let dir = tempfile::tempdir().unwrap();
    let original = guarded(false);
    let parts: Vec<&[u8]> = original.iter().map(Vec::as_slice).collect();
    let path = object(dir.path(), "f.obj", &parts, &[], None);
    let output = rename(&path, "0:ebx:ebp,2:ebx:ebp,4:ebx:ebp");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(function(&path).0, guarded(true).concat());
}

/// An object whose `.text` holds `_f`, `code`, then `_h`, `mov eax, [_g]`
/// and `ret`, with a `DIR32` relocation to `_g` in `_h` and FPO data that
/// holds `_f`'s size.
fn followed(dir: &Path, name: &str, code: &[&[u8]]) -> std::path::PathBuf {
    let mut file = Object::new(BinaryFormat::Coff, Architecture::I386, Endianness::Little);
    file.set_mangling(Mangling::None);
    let text = file.section_id(StandardSection::Text);
    let code = code.concat();
    file.append_section_data(text, &[&code[..], &[0xa1, 0, 0, 0, 0, 0xc3]].concat(), 16);
    let symbol = |name: &[u8], value, section| Symbol {
        name: name.to_vec(),
        value,
        size: 0,
        kind: SymbolKind::Text,
        scope: SymbolScope::Linkage,
        weak: false,
        section,
        flags: SymbolFlags::None,
    };
    let function = file.add_symbol(symbol(b"_f", 0, SymbolSection::Section(text)));
    let after = u64::try_from(code.len()).unwrap();
    file.add_symbol(symbol(b"_h", after, SymbolSection::Section(text)));
    let global = file.add_symbol(symbol(b"_g", 0, SymbolSection::Undefined));
    file.add_relocation(
        text,
        Relocation {
            offset: after + 1,
            symbol: global,
            addend: 0,
            flags: RelocationFlags::Coff {
                typ: object::pe::IMAGE_REL_I386_DIR32,
            },
        },
    )
    .unwrap();
    let fpo = file.add_section(Vec::new(), b".debug$F".to_vec(), object::SectionKind::Debug);
    let mut entry = vec![0; 16];
    entry[4] = u8::try_from(code.len()).unwrap();
    file.append_section_data(fpo, &entry, 4);
    file.add_relocation(
        fpo,
        Relocation {
            offset: 0,
            symbol: function,
            addend: 0,
            flags: RelocationFlags::Coff {
                typ: object::pe::IMAGE_REL_I386_DIR32NB,
            },
        },
    )
    .unwrap();
    let path = dir.join(name);
    std::fs::write(&path, file.write().unwrap()).unwrap();
    path
}

#[test]
fn a_longer_function_without_padding_moves_what_follows_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = followed(
        dir.path(),
        "f.obj",
        &[&LOAD_EBX, &READ_EBX, &CLEAR_BOTH, &RET],
    );
    let output = rename(&path, "0:ebx:ebp,1:ebx:ebp");
    assert!(output.status.success(), "{}", stderr(&output));
    let bytes = std::fs::read(&path).unwrap();
    let file = object::File::parse(&*bytes).unwrap();
    let follower = file.symbol_by_name("_h").unwrap();
    let text = file
        .section_by_index(follower.section_index().unwrap())
        .unwrap();
    assert_eq!(follower.address(), 12);
    assert_eq!(
        text.data().unwrap(),
        [
            &LOAD_EBP[..],
            &READ_EBP,
            &CLEAR_BOTH,
            &RET,
            &[0xa1, 0, 0, 0, 0, 0xc3]
        ]
        .concat()
    );
    let offsets: Vec<u64> = text.relocations().map(|(offset, _)| offset).collect();
    assert_eq!(offsets, [13]);
    let fpo = file.section_by_name(".debug$F").unwrap();
    assert_eq!(fpo.data().unwrap()[4], 12, "FPO data holds the new size");
}

#[test]
fn propose_finds_a_rename_that_changes_the_length() {
    let dir = tempfile::tempdir().unwrap();
    let (ours, theirs) = (guarded(false), guarded(true));
    let built = object(
        dir.path(),
        "built.obj",
        &ours.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        &[],
        None,
    );
    let reference = object(
        dir.path(),
        "reference.obj",
        &theirs.iter().map(Vec::as_slice).collect::<Vec<_>>(),
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
            "0:ebx:ebp,2:ebx:ebp,4:ebx:ebp"
        ]]
    );
}

#[test]
fn a_rename_in_an_object_from_msvc_undoes_through_its_proposal() {
    let dir = tempfile::tempdir().unwrap();
    let reference =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/grind/reference/reference.obj");
    let built = dir.path().join("built.obj");
    std::fs::copy(&reference, &built).unwrap();
    // `ebx` and `ebp` trade places from the loop's second load to its first
    // `add`, in a function with line numbers, `.lf`, CodeView, and FPO data.
    let swap = |path: &Path, map: &str| {
        addon(&[
            "rewrite",
            path.to_str().unwrap(),
            "rename",
            "--symbol",
            "_transform",
            "--map",
            map,
        ])
    };
    let output = swap(
        &built,
        "14:ebp:ebx,15:ebp:ebx,18:ebx:ebp,19:ebx:ebp,21:ebx:ebp,21:ebp:ebx,22:ebx:ebp",
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let output = addon(&[
        "propose",
        built.to_str().unwrap(),
        reference.to_str().unwrap(),
        "_transform",
    ]);
    let proposals: Vec<Vec<String>> = serde_json::from_slice(&output.stdout).unwrap();
    let [proposal] = &proposals[..] else {
        panic!("{proposals:?}");
    };
    assert!(swap(&built, &proposal[4]).status.success());
    assert_eq!(
        std::fs::read(&built).unwrap(),
        std::fs::read(&reference).unwrap()
    );
}
