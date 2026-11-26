#![cfg(test)]
//! Behaviour of the add-on's rewrites and proposals, through its binary.

mod exchange;
mod region;
mod rename;
mod schedule;

use std::path::{Path, PathBuf};
use std::process::Command;

use object::write::{Mangling, Object, Relocation, StandardSection, Symbol, SymbolSection};
use object::{
    Architecture, BinaryFormat, Endianness, ObjectSection, ObjectSymbol, RelocationFlags,
    SymbolFlags, SymbolKind, SymbolScope,
};

/// An i386 COFF object holding `_f` with `code`, a `DIR32` relocation to
/// `_g` at each of `loads`, and, with `prologue`, FPO data for `_f`.
pub(crate) fn object(
    dir: &Path,
    name: &str,
    code: &[&[u8]],
    loads: &[u64],
    prologue: Option<u8>,
) -> PathBuf {
    entered(dir, name, code, loads, prologue, None)
}

/// `object`, with a data word that holds `_f`'s address plus `entry`, as a
/// table of places to jump to would.
pub(crate) fn entered(
    dir: &Path,
    name: &str,
    code: &[&[u8]],
    loads: &[u64],
    prologue: Option<u8>,
    entry: Option<u32>,
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
    if let Some(entry) = entry {
        let data = file.section_id(StandardSection::Data);
        file.append_section_data(data, &entry.to_le_bytes(), 4);
        file.add_relocation(
            data,
            Relocation {
                offset: 0,
                symbol: function,
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
