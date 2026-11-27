//! The MCP server: the tool list agents load, and a session that sets up and
//! verifies a project over stdin and stdout.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Stdio;

use serde_json::{Value, json};

use super::World;
use super::locate::archive;
use super::score::{ZERO, object};

/// Sends `requests` to one server session and returns each reply's `result`.
fn session(world: &World, requests: &[(&str, Value)]) -> Vec<Value> {
    let mut server = world
        .command()
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("server starts");
    let mut input = server.stdin.take().expect("stdin");
    for (id, (method, params)) in requests.iter().enumerate() {
        writeln!(
            input,
            "{}",
            json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
        )
        .expect("request");
    }
    drop(input);
    let replies: Vec<Value> = BufReader::new(server.stdout.take().expect("stdout"))
        .lines()
        .map(|line| serde_json::from_str::<Value>(&line.expect("reply")).expect("reply is JSON"))
        .map(|reply| reply["result"].clone())
        .collect();
    assert!(server.wait().expect("server exits").success());
    replies
}

fn text(reply: &Value) -> String {
    reply["content"][0]["text"]
        .as_str()
        .expect("text content")
        .to_owned()
}

/// MCP-1: the result arrives once, as compact JSON text.
#[test]
fn an_mcp_reply_carries_its_result_once() {
    let world = World::new();
    let dir = world.project().to_string_lossy().into_owned();
    let replies = session(
        &world,
        &[
            (
                "tools/call",
                json!({ "name": "setup", "arguments": { "dir": dir } }),
            ),
            (
                "tools/call",
                json!({ "name": "verify", "arguments": { "dir": dir } }),
            ),
        ],
    );
    let verify = &replies[1];
    let result: Value = serde_json::from_str(&text(verify)).expect("text is JSON");
    assert_eq!(
        (&result["identical"], verify.get("structuredContent")),
        (&json!(true), None),
        "{verify}"
    );
    assert_eq!(verify["content"].as_array().map(Vec::len), Some(1));
}

/// MCP-2: a result over the budget becomes an error that says how to get it.
#[test]
fn an_mcp_reply_over_the_budget_names_the_command_to_run_instead() {
    let world = World::new();
    fs::write(
        world.project().join("image.obj"),
        object(&[("_image", &[0xcc; 8], None)]),
    )
    .expect("image");
    let members: Vec<(String, Vec<u8>)> = (0..400)
        .map(|index| {
            let name = format!("_library_function_number_{index:04}");
            (
                format!("m{index:04}.obj"),
                object(&[(name.as_str(), ZERO, None)]),
            )
        })
        .collect();
    let members: Vec<(&str, Vec<u8>)> = members
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.clone()))
        .collect();
    fs::write(world.project().join("sdk.a"), archive(&members)).expect("library");
    let root = world.project();
    let replies = session(
        &world,
        &[(
            "tools/call",
            json!({ "name": "locate", "arguments": {
                "binary": root.join("image.obj"),
                "libraries": [root.join("sdk.a")],
                "all": true,
            } }),
        )],
    );
    let reply = &replies[0];
    let message = text(reply);
    assert_eq!(
        reply["isError"],
        true,
        "{}",
        &message[..message.len().min(300)]
    );
    assert!(
        message.len() <= 22_000
            && message.contains("rbit --json locate")
            && message.contains("all"),
        "{message}"
    );
}

/// MCP-3: annotations say which tools only read and which delete.
#[test]
fn the_mcp_tools_declare_what_they_change() {
    let world = World::new();
    let replies = session(&world, &[("tools/list", json!({}))]);
    let hint = |name: &str, key: &str| {
        replies[0]["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .find(|tool| tool["name"] == name)
            .map_or(Value::Null, |tool| tool["annotations"][key].clone())
    };
    let read_only: Vec<Value> = ["status", "diff", "locate", "space", "score", "setup"]
        .iter()
        .map(|name| hint(name, "readOnlyHint"))
        .collect();
    assert_eq!(
        read_only,
        [true, true, true, true, false, false].map(Value::from)
    );
    assert_eq!(
        (
            hint("gc", "destructiveHint"),
            hint("tool_remove", "destructiveHint")
        ),
        (json!(true), json!(true))
    );
}

/// The tool list every agent loads into its context, pinned so a change to a
/// description or argument shows in review with its size.
#[test]
fn the_mcp_tool_list_is_the_published_one() {
    let world = World::new();
    let mut server = world
        .command()
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("server starts");
    let mut input = server.stdin.take().expect("stdin");
    writeln!(
        input,
        "{}",
        json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} })
    )
    .expect("request");
    drop(input);
    let mut line = String::new();
    BufReader::new(server.stdout.take().expect("stdout"))
        .read_line(&mut line)
        .expect("reply");
    assert!(server.wait().expect("server exits").success());
    let tools =
        serde_json::from_str::<Value>(&line).expect("reply is JSON")["result"]["tools"].clone();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/mcp-tools.json");
    let rendered = serde_json::to_string_pretty(&tools).expect("tools serialize") + "\n";
    if std::env::var_os("REPROBIT_BLESS").is_some() {
        fs::write(&path, &rendered).expect("write mcp-tools.json");
    }
    assert_eq!(
        fs::read_to_string(&path).unwrap_or_default(),
        rendered,
        "schemas/mcp-tools.json is stale; rerun this test with REPROBIT_BLESS=1 and review the diff"
    );
}

#[test]
fn the_mcp_server_answers_both_handshakes_and_runs_commands() {
    let world = World::new();
    let mut server = world
        .command()
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("server starts");
    let mut input = server.stdin.take().expect("stdin");
    let mut output = BufReader::new(server.stdout.take().expect("stdout"));
    let mut ask = |id: u32, method: &str, params: Value| -> Value {
        writeln!(
            input,
            "{}",
            json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
        )
        .expect("request");
        let mut line = String::new();
        output.read_line(&mut line).expect("reply");
        let reply: Value = serde_json::from_str(&line).expect("reply is JSON");
        assert_eq!(reply["id"], id);
        reply
    };

    assert_eq!(
        ask(1, "initialize", json!({ "protocolVersion": "2025-06-18" }))["result"]["protocolVersion"],
        "2025-06-18"
    );
    assert!(
        ask(2, "server/discover", json!({}))["result"]["supportedVersions"]
            .as_array()
            .is_some_and(|versions| versions.contains(&json!("2026-07-28")))
    );
    let listed = ask(3, "tools/list", json!({}));
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(
        names.contains(&"verify")
            && names.contains(&"tool_install")
            && names.contains(&"gc")
            && names.contains(&"score"),
        "{names:?}"
    );

    let dir = world.project().to_string_lossy().into_owned();
    let setup = ask(
        4,
        "tools/call",
        json!({ "name": "setup", "arguments": { "dir": dir } }),
    );
    assert_eq!(setup["result"]["isError"], false, "{setup}");
    let verify = ask(
        5,
        "tools/call",
        json!({ "name": "verify", "arguments": { "dir": dir } }),
    );
    assert_eq!(
        serde_json::from_str::<Value>(&text(&verify["result"])).expect("text is JSON")["identical"],
        true,
        "{verify}"
    );
    assert!(ask(6, "nonsense", json!({}))["error"].is_object());

    drop(input);
    assert!(server.wait().expect("server exits").success());
}
