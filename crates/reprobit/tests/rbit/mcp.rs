//! The MCP server: the tool list agents load, and a session that sets up and
//! verifies a project over stdin and stdout.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Stdio;

use serde_json::{Value, json};

use super::World;

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
        verify["result"]["structuredContent"]["identical"], true,
        "{verify}"
    );
    assert!(ask(6, "nonsense", json!({}))["error"].is_object());

    drop(input);
    assert!(server.wait().expect("server exits").success());
}
