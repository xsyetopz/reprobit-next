//! `rbit mcp`: the CLI's commands as MCP tools over stdio. Each tool call is
//! parsed as the equivalent command line and returns that command's `--json`
//! value, so agents and people see one behaviour. Answers both the stateless
//! 2026-07-28 protocol (`server/discover`) and the older `initialize` handshake.

use std::io::{self, BufRead, Write};

use anyhow::Result;
use clap::Parser;
use serde_json::{Map, Value, json};

use crate::cli::{self, Cli};
use crate::ui;

const VERSIONS: &[&str] = &["2026-07-28", "2025-11-25", "2025-06-18", "2025-03-26"];
const INSTRUCTIONS: &str = "ReproBit rebuilds binaries byte for byte with their original compilers. \
Call status first; it names the next step. verify is the only call that certifies a match.";

/// How a tool argument reaches the command line.
#[derive(Clone, Copy)]
enum Arg {
    /// The positional project directory.
    Dir,
    /// A positional value or values.
    Positional,
    /// `--name value`, repeated for arrays.
    Flag,
    /// `--name` when true.
    Switch,
}

/// `(tool, command words, description, [(argument, kind, json type, required, description)])`.
type Spec = (
    &'static str,
    &'static [&'static str],
    &'static str,
    &'static [(&'static str, Arg, &'static str, bool, &'static str)],
);

const DIR: (&str, Arg, &str, bool, &str) = (
    "dir",
    Arg::Dir,
    "string",
    false,
    "Project directory; default the server's working directory.",
);

const TOOLS: &[Spec] = &[
    (
        "status",
        &["status"],
        "What is ready in a project and the next thing to do.",
        &[DIR],
    ),
    (
        "setup",
        &["setup"],
        "Install the project's compiler and tools, and pin them in reprobit.lock.",
        &[DIR],
    ),
    ("build", &["build"], "Incremental developer build.", &[DIR]),
    (
        "verify",
        &["verify"],
        "Cold build and literal comparison of every target; writes the trust report.",
        &[DIR],
    ),
    (
        "diff",
        &["diff"],
        "Where the last build differs from the references.",
        &[DIR],
    ),
    (
        "repair",
        &["repair"],
        "After a source edit, drop each saved prelude the build no longer needs.",
        &[DIR],
    ),
    (
        "grind",
        &["grind"],
        "Search declaration-only preludes until one function matches a reference object.",
        &[
            DIR,
            ("step", Arg::Flag, "string", true, "Compile step id."),
            ("symbol", Arg::Flag, "string", true, "Function symbol."),
            (
                "reference_object",
                Arg::Flag,
                "string",
                true,
                "Reference object path.",
            ),
            (
                "family",
                Arg::Flag,
                "array",
                false,
                "Families: shape, pad, forward, extern.",
            ),
            (
                "limit",
                Arg::Flag,
                "integer",
                false,
                "Most candidates to compile.",
            ),
            (
                "accept",
                Arg::Switch,
                "boolean",
                false,
                "Save the match into reprobit.toml.",
            ),
        ],
    ),
    (
        "tool_list",
        &["tool", "list"],
        "Search the tool and compiler catalog.",
        &[
            (
                "filter",
                Arg::Positional,
                "string",
                false,
                "Text in id, summary, or tags.",
            ),
            (
                "installed",
                Arg::Switch,
                "boolean",
                false,
                "Only installed tools.",
            ),
        ],
    ),
    (
        "tool_info",
        &["tool", "info"],
        "One catalog entry and its install state.",
        &[("id", Arg::Positional, "string", true, "Tool id.")],
    ),
    (
        "tool_install",
        &["tool", "install"],
        "Download, check, and unpack tools.",
        &[("ids", Arg::Positional, "array", true, "Tool ids.")],
    ),
    (
        "tool_remove",
        &["tool", "remove"],
        "Delete installed tools.",
        &[("ids", Arg::Positional, "array", true, "Tool ids.")],
    ),
    (
        "schema",
        &["schema"],
        "JSON Schema for reprobit.toml (project) or the catalog.",
        &[(
            "name",
            Arg::Positional,
            "string",
            true,
            "project or catalog.",
        )],
    ),
];

fn tool_list() -> Value {
    let tools: Vec<Value> = TOOLS
        .iter()
        .map(|(name, _, description, args)| {
            let mut properties = Map::new();
            for (arg, _, kind, _, about) in *args {
                let mut schema = json!({ "type": kind, "description": about });
                if *kind == "array" {
                    schema["items"] = json!({ "type": "string" });
                }
                properties.insert((*arg).into(), schema);
            }
            let required: Vec<&str> = args.iter().filter(|item| item.3).map(|item| item.0).collect();
            json!({
                "name": name,
                "description": description,
                "inputSchema": { "type": "object", "properties": properties, "required": required, "additionalProperties": false },
            })
        })
        .collect();
    json!({ "resultType": "complete", "tools": tools })
}

fn argv(
    words: &[&str],
    args: &[(&str, Arg, &str, bool, &str)],
    given: &Map<String, Value>,
) -> Vec<String> {
    let mut argv = vec!["rbit".to_owned(), "--quiet".to_owned()];
    argv.extend(words.iter().map(|word| (*word).to_owned()));
    let text = |value: &Value| {
        value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned)
    };
    for (name, kind, ..) in args {
        let Some(value) = given.get(*name) else {
            continue;
        };
        let values: Vec<String> = value.as_array().map_or_else(
            || vec![text(value)],
            |items| items.iter().map(text).collect(),
        );
        let flag = format!("--{}", name.replace('_', "-"));
        match kind {
            Arg::Dir | Arg::Positional => argv.extend(values),
            Arg::Flag => values
                .into_iter()
                .for_each(|value| argv.extend([flag.clone(), value])),
            Arg::Switch if value.as_bool() == Some(true) => argv.push(flag),
            Arg::Switch => {}
        }
    }
    argv
}

fn call(params: &Value) -> Value {
    let name = params["name"].as_str().unwrap_or_default();
    let given = params["arguments"].as_object().cloned().unwrap_or_default();
    let Some((_, words, _, args)) = TOOLS.iter().find(|tool| tool.0 == name) else {
        return json!({ "resultType": "complete", "isError": true, "content": [{ "type": "text", "text": format!("no tool {name}") }] });
    };
    let outcome = Cli::try_parse_from(argv(words, args, &given))
        .map_err(|error| anyhow::anyhow!("{error}"))
        .and_then(|parsed| cli::run(parsed.command));
    let (ok, value) = match outcome {
        Ok(outcome) => (outcome.ok, outcome.json),
        Err(error) => (false, json!({ "error": format!("{error:#}") })),
    };
    let structured = if value.is_object() {
        value.clone()
    } else {
        json!({ "result": value })
    };
    json!({
        "resultType": "complete",
        "isError": !ok,
        "content": [{ "type": "text", "text": value.to_string() }],
        "structuredContent": structured,
    })
}

fn server_info() -> Value {
    json!({ "name": "reprobit", "version": env!("CARGO_PKG_VERSION") })
}

fn respond(request: &Value) -> Option<Value> {
    let id = request.get("id")?.clone();
    let params = &request["params"];
    let result = match request["method"].as_str().unwrap_or_default() {
        "initialize" => {
            let asked = params["protocolVersion"].as_str().unwrap_or_default();
            let version = if VERSIONS.contains(&asked) {
                asked
            } else {
                VERSIONS[1]
            };
            json!({ "protocolVersion": version, "capabilities": { "tools": {} }, "serverInfo": server_info(), "instructions": INSTRUCTIONS })
        }
        "server/discover" => json!({
            "resultType": "complete",
            "supportedVersions": VERSIONS,
            "capabilities": { "tools": {} },
            "instructions": INSTRUCTIONS,
            "_meta": { "io.modelcontextprotocol/serverInfo": server_info() },
        }),
        "tools/list" => tool_list(),
        "tools/call" => call(params),
        "ping" => json!({}),
        method => {
            return Some(
                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": format!("unknown method {method}") } }),
            );
        }
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

/// Serves requests until stdin closes. Progress goes to stderr, never stdout.
pub(crate) fn serve() -> Result<()> {
    ui::set_quiet(true);
    let stdout = io::stdout();
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(request) => respond(&request),
            Err(error) => Some(
                json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": error.to_string() } }),
            ),
        };
        if let Some(reply) = reply {
            let mut out = stdout.lock();
            writeln!(out, "{reply}")?;
            out.flush()?;
        }
    }
    Ok(())
}
