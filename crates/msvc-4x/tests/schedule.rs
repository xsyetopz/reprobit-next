#![cfg(test)]
//! Behaviour of the `schedule` rewrite and its proposals, through the add-on binary.

use std::path::{Path, PathBuf};
use std::process::Command;

use object::write::{Mangling, Object, Relocation, StandardSection, Symbol, SymbolSection};
use object::{
    Architecture, BinaryFormat, Endianness, ObjectSection, ObjectSymbol, RelocationFlags,
    SymbolFlags, SymbolKind, SymbolScope,
};

/// `mov eax, [_g]`, relocated at +1.
const LOAD: [u8; 5] = [0xa1, 0, 0, 0, 0];
/// `mov ecx, 5`
const SET_ECX: [u8; 5] = [0xb9, 5, 0, 0, 0];
/// `mov edx, 1`
const SET_EDX: [u8; 5] = [0xba, 1, 0, 0, 0];
/// `mov ebx, 2`
const SET_EBX: [u8; 5] = [0xbb, 2, 0, 0, 0];
const RET: [u8; 1] = [0xc3];

/// An i386 COFF object holding `_f` with `code`, a `DIR32` relocation to
/// `_g` at each of `loads`, and, with `prologue`, FPO data for `_f`.
pub(crate) fn object(
    dir: &Path,
    name: &str,
    code: &[&[u8]],
    loads: &[u64],
    prologue: Option<u8>,
) -> PathBuf {
    let mut file = Object::new(BinaryFormat::Coff, Architecture::I386, Endianness::Little);
    file.set_mangling(Mangling::None);
    let text = file.section_id(StandardSection::Text);
    file.append_section_data(text, &code.concat(), 16);
    let function = file.add_symbol(Symbol {
        name: b"_f".to_vec(),
        value: 0,
        size: 0,
        kind: SymbolKind::Text,
        scope: SymbolScope::Linkage,
        weak: false,
        section: SymbolSection::Section(text),
        flags: SymbolFlags::None,
    });
    let global = file.add_symbol(Symbol {
        name: b"_g".to_vec(),
        value: 0,
        size: 0,
        kind: SymbolKind::Data,
        scope: SymbolScope::Linkage,
        weak: false,
        section: SymbolSection::Undefined,
        flags: SymbolFlags::None,
    });
    for &offset in loads {
        file.add_relocation(
            text,
            Relocation {
                offset,
                symbol: global,
                addend: 0,
                flags: RelocationFlags::Coff {
                    typ: object::pe::IMAGE_REL_I386_DIR32,
                },
            },
        )
        .unwrap();
    }
    if let Some(prologue) = prologue {
        let fpo = file.add_section(Vec::new(), b".debug$F".to_vec(), object::SectionKind::Debug);
        let mut entry = vec![0; 16];
        entry[4] = u8::try_from(code.concat().len()).unwrap();
        entry[14] = prologue;
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
    }
    let path = dir.join(name);
    std::fs::write(&path, file.write().unwrap()).unwrap();
    path
}

/// `_f`'s bytes and the offsets of its relocations.
pub(crate) fn function(path: &Path) -> (Vec<u8>, Vec<u64>) {
    let bytes = std::fs::read(path).unwrap();
    let file = object::File::parse(&*bytes).unwrap();
    let symbol = object::Object::symbol_by_name(&file, "_f").unwrap();
    let section = object::Object::section_by_index(&file, symbol.section_index().unwrap()).unwrap();
    (
        section.data().unwrap().to_vec(),
        section.relocations().map(|(offset, _)| offset).collect(),
    )
}

pub(crate) fn addon(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rbit-msvc-4x"))
        .args(args)
        .output()
        .unwrap()
}

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
