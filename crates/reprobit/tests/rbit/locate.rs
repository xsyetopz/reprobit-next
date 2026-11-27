//! Finding original library objects inside a linked binary. The archive and the
//! "binary" are built here: a COFF object's `.text` stands in for linked code.

use std::fs;

use super::score::{LOAD, ONE, TWO, ZERO, object};
use super::{Run, World, succeeds};

/// A GNU `ar` archive of `(name, bytes)` members.
fn archive(members: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"!<arch>\n".to_vec();
    for (name, data) in members {
        let header = format!(
            "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
            format!("{name}/"),
            0,
            0,
            0,
            644,
            data.len()
        );
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(data);
        if data.len() % 2 == 1 {
            out.push(b'\n');
        }
    }
    out
}

/// Runs `rbit locate` on an image whose code is
/// `cc×4, _load (operand filled), _two, _zero, cc, _zero, cc`.
fn locate() -> Run {
    let world = World::new();
    let filled: &[u8] = &[0xa1, 0x78, 0x56, 0x34, 0x12, 0xc3];
    let code = [&[0xcc; 4][..], filled, TWO, ZERO, &[0xcc], ZERO, &[0xcc]].concat();
    fs::write(
        world.project().join("image.obj"),
        object(&[("_image", &code, None)]),
    )
    .expect("image");
    let library = archive(&[
        (
            "a.obj",
            object(&[("_load", LOAD, Some("_global")), ("_two", TWO, None)]),
        ),
        ("b.obj", object(&[("_one", ONE, None)])),
        ("c.obj", object(&[("_zero", ZERO, None)])),
    ]);
    fs::write(world.project().join("sdk.a"), library).expect("library");
    world.rbit(&["locate", "image.obj", "sdk.a"])
}

fn function<'a>(run: &'a Run, member: &str, name: &str) -> &'a serde_json::Value {
    succeeds(run)["members"]
        .as_array()
        .expect("members")
        .iter()
        .find(|item| item["member"] == member)
        .and_then(|item| item["functions"].as_array())
        .and_then(|functions| functions.iter().find(|item| item["name"] == name))
        .unwrap_or_else(|| panic!("{member} {name} in {}", run.json))
}

fn linked(run: &Run, member: &str) -> bool {
    run.json["members"]
        .as_array()
        .expect("members")
        .iter()
        .find(|item| item["member"] == member)
        .is_some_and(|item| item["linked"] == true)
}

/// Runs `rbit locate` on an image of `code` and an archive of `(member, functions)`.
/// `(member, [(function, code, symbol its offset 1 loads)])`.
type Members<'a> = [(&'a str, &'a [(&'a str, &'a [u8], Option<&'a str>)])];

fn locate_in(code: &[u8], members: &Members<'_>) -> Run {
    let world = World::new();
    fs::write(
        world.project().join("image.obj"),
        object(&[("_image", code, None)]),
    )
    .expect("image");
    let members: Vec<(&str, Vec<u8>)> = members
        .iter()
        .map(|(name, functions)| (*name, object(functions)))
        .collect();
    fs::write(world.project().join("sdk.a"), archive(&members)).expect("library");
    world.rbit(&["locate", "image.obj", "sdk.a"])
}

/// LOCATE-1: a function placed once is found at its address despite its filled relocation.
#[test]
fn locate_finds_a_function_placed_once_at_its_address() {
    let run = locate();
    let load = function(&run, "a.obj", "_load");
    assert_eq!(
        (&load["status"], &load["address"]),
        (&"found".into(), &4.into())
    );
}

/// LOCATE-2: a member is linked only when every function it defines is found.
#[test]
fn locate_counts_a_member_linked_only_when_all_its_functions_are_found() {
    let run = locate();
    assert_eq!(function(&run, "a.obj", "_two")["address"], 10);
    assert_eq!(function(&run, "b.obj", "_one")["status"], "absent");
    assert_eq!(
        (linked(&run, "a.obj"), linked(&run, "b.obj")),
        (true, false)
    );
    assert_eq!(run.json["linked"], 1);
}

/// LOCATE-3: a function at several places is ambiguous, and its member is not linked.
#[test]
fn locate_reports_a_function_at_several_places_as_ambiguous() {
    let run = locate();
    let zero = function(&run, "c.obj", "_zero");
    assert_eq!(
        (&zero["status"], &zero["places"]),
        (&"ambiguous".into(), &2.into())
    );
    assert!(!linked(&run, "c.obj"));
}

/// LOCATE-4: a shared place goes to the one claimant whose member is otherwise found.
#[test]
fn locate_gives_a_shared_place_only_to_a_member_found_in_full() {
    let code = [&[0xcc][..], TWO, ONE, &[0xcc]].concat();
    let run = locate_in(
        &code,
        &[
            ("p.obj", &[("_p", TWO, None), ("_s", ONE, None)]),
            ("q.obj", &[("_q", ONE, None), ("_z", ZERO, None)]),
        ],
    );
    assert_eq!(function(&run, "p.obj", "_s")["status"], "found");
    let q = function(&run, "q.obj", "_q");
    assert_eq!(q["status"], "ambiguous");
    assert!(
        q["shared"]
            .as_array()
            .is_some_and(|shared| shared.iter().any(|item| item == "p.obj:_s")),
        "{q}"
    );
    assert_eq!(
        (linked(&run, "p.obj"), linked(&run, "q.obj")),
        (true, false)
    );

    let twins = locate_in(
        &code,
        &[
            ("x.obj", &[("_x", ONE, None)]),
            ("y.obj", &[("_y", ONE, None)]),
        ],
    );
    assert_eq!(
        (
            &function(&twins, "x.obj", "_x")["status"],
            &function(&twins, "y.obj", "_y")["status"]
        ),
        (&"ambiguous".into(), &"ambiguous".into())
    );
    assert_eq!(twins.json["linked"], 0);
}
