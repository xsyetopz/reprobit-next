//! Register renames: found by grind, refused when unsound, and carried into
//! debug records. The objects are built here, so no compiler is needed.

use std::fs;

use object::{
    Architecture, BinaryFormat, Endianness, Object, ObjectSection, RelocationFlags, SectionKind,
    SymbolFlags, SymbolKind, SymbolScope, write,
};
use serde_json::json;
use sha2::{Digest, Sha256};

use super::{World, succeeds};

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
        json!({ "prelude": null, "registers": { "esi": "edi", "edi": "esi" }, "rewrite": [] })
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
fn grind_saves_an_add_on_rewrite_that_makes_the_function_match() {
    let world = object_world(
        "main.obj",
        &coff_function(SAVES_ESI_FIRST),
        &coff_function(SAVES_EDI_FIRST),
    );
    fs::write(
        world.project().join("fixed.obj"),
        coff_function(SAVES_EDI_FIRST),
    )
    .expect("fixed");
    // Proposes one rewrite, which copies a project file over the object.
    let (name, script): (&str, &[u8]) = if cfg!(windows) {
        (
            "addon.cmd",
            b"@echo off\r\nif \"%1\"==\"propose\" (echo [[\"copy\",\"fixed.obj\"]] & exit /b 0)\r\ncopy /b /y \"%~4\" \"%~2\" >nul\r\n",
        )
    } else {
        (
            "addon",
            b"#!/bin/sh\nif [ \"$1\" = propose ]; then echo '[[\"copy\",\"fixed.obj\"]]'; exit 0; fi\ncp \"$4\" \"$2\"\n",
        )
    };
    let mirror = world.temp.path().join("mirror").join(name);
    fs::write(&mirror, script).expect("add-on");
    let catalog = world.temp.path().join("catalog.toml");
    let entry = format!(
        "[tools.fake-addon]\nkind = \"addon\"\nversion = \"1\"\nruns = \"host\"\nbin = {{ fake-addon = \"{name}\" }}\n\
         [[tools.fake-addon.get]]\nurl = {url:?}\nsha256 = \"{sha}\"\nas = \"{name}\"\n",
        url = mirror.display().to_string(),
        sha = hex::encode(Sha256::digest(script)),
    );
    fs::write(
        &catalog,
        fs::read_to_string(&catalog).expect("catalog") + &entry,
    )
    .expect("catalog");
    let project = fs::read_to_string(world.project().join("reprobit.toml")).expect("project");
    world.edit(
        "reprobit.toml",
        &project.replace(
            "compiler = \"fake-cc\"",
            "compiler = \"fake-cc\"\ntools = [\"fake-addon\"]",
        ),
    );
    succeeds(&world.rbit(&["setup"]));
    let grind = [
        "grind",
        "--step",
        "compile.main",
        "--symbol",
        "_f",
        "--reference-object",
        "reference/main.obj",
        "--family",
        "fake-addon",
        "--accept",
    ];

    let found = world.rbit(&grind);

    assert_eq!(
        succeeds(&found)["found"]["rewrite"],
        json!([{ "addon": "fake-addon", "args": ["copy", "fixed.obj"] }])
    );
    let verified = world.rbit(&["verify"]);
    let adjustment = succeeds(&verified)["steps"][0]["adjustments"][0]
        .as_str()
        .expect("listed");
    assert!(
        adjustment.starts_with("rewrite fake-addon copy fixed.obj"),
        "{adjustment}"
    );
}
