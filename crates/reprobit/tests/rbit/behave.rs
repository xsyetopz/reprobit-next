//! Behaviour checks: a recompiled target agrees with its reference when the
//! same command gives the same results, though its bytes differ. The
//! "binaries" are shell scripts the fake compiler copies.

use super::{World, succeeds};

fn project(check: &str) -> String {
    format!(
        r#"
name = "recompiled"

[toolchain]
compiler = "fake-cc"

[[step]]
id = "compile.main"
program = "cc"
args = ["${{SOURCE}}/main.c", "app.bin"]
inputs = ["source/main.c"]
outputs = ["app.bin"]

[[target]]
id = "app"
artifact = "app.bin"
reference = "reference/app.bin"

{check}
"#
    )
}

const GREETS: &str = r#"
[[check]]
id = "greets"
target = "app"
run = ["sh", "${BINARY}", "world"]
stdin = "input.txt"
"#;

/// A world whose reference reads a line and greets with it.
fn world(check: &str, artifact: &str) -> World {
    let world = World::new();
    world.edit("reprobit.toml", &project(check));
    world.edit("reference/app.bin", "read line; echo \"hello $1 $line\"\n");
    world.edit("input.txt", "from stdin\n");
    world.edit("main.c", artifact);
    succeeds(&world.rbit(&["setup"]));
    world
}

#[test]
fn a_recompiled_target_with_other_bytes_and_the_same_output_agrees() {
    let world = world(
        GREETS,
        "read text\nprintf 'hello %s %s\\n' \"$1\" \"$text\"\n",
    );
    assert_eq!(world.rbit(&["verify"]).code, 1, "the bytes differ");

    let run = world.rbit(&["behave"]);
    let json = succeeds(&run);
    assert_eq!(json["agree"], true);
    assert_eq!(
        json["checks"][0]["artifact"]["stdout"],
        "hello world from stdin\n"
    );
    assert_eq!(
        json["checks"][0]["reference"]["stdout_sha256"],
        json["checks"][0]["artifact"]["stdout_sha256"]
    );
}

#[test]
fn different_output_or_a_hang_fails_the_check() {
    let changed = world(GREETS, "read line; echo \"goodbye $1\"\n");
    let run = changed.rbit(&["behave"]);
    assert_eq!(run.code, 1, "{}", run.stderr);
    assert_eq!(
        run.json["checks"][0]["differs"],
        serde_json::json!(["stdout"])
    );

    let hangs = world(
        &GREETS.replace("stdin = \"input.txt\"", "timeout = 1"),
        "sleep 30\n",
    );
    let run = hangs.rbit(&["behave", "--check", "greets"]);
    assert_eq!(run.code, 1, "{}", run.stderr);
    assert_eq!(run.json["checks"][0]["artifact"]["timed_out"], true);
    assert_eq!(run.json["checks"][0]["agree"], false);
}

#[test]
fn a_check_must_name_a_target_and_pass_the_binary() {
    let world = World::new();
    world.edit(
        "reprobit.toml",
        &project("[[check]]\nid = \"x\"\ntarget = \"nope\"\nrun = [\"sh\", \"${BINARY}\"]\n"),
    );
    let run = world.rbit(&["behave"]);
    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("target nope"), "{}", run.stderr);

    world.edit(
        "reprobit.toml",
        &project("[[check]]\nid = \"x\"\ntarget = \"app\"\nrun = [\"true\"]\n"),
    );
    let run = world.rbit(&["behave"]);
    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("BINARY"), "{}", run.stderr);
}
