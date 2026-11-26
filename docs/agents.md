# Agents

`rbit` is the interface. Every surface below runs the same commands and returns the same
JSON, so an agent's evidence is what a person would see.

## CLI and skill

`rbit --json <command>` prints one JSON value on stdout; progress goes to stderr, and `-q`
silences it. Exit codes: 0 success, 1 a negative result (a mismatch, a status with work
left, a grind with no match), 2 an error, whose JSON is `{"error": "…"}`.

[`skills/reprobit/SKILL.md`](../skills/reprobit/SKILL.md) teaches the loop: `status`, then
`setup`, `build`, `diff`, `grind`, and `verify`. It uses only portable frontmatter, so any
agent that reads Agent Skills can load it.

## Claude Code

```sh
claude plugin marketplace add xsyetopz/reprobit-next
claude plugin install reprobit@reprobit
```

The plugin brings the skill and registers `rbit mcp` from [`.mcp.json`](../.mcp.json).

## Codex

The repository is also a Codex plugin: [`.codex-plugin/plugin.json`](../.codex-plugin/plugin.json)
points at the same `skills/` and `.mcp.json`, and
[`.agents/plugins/marketplace.json`](../.agents/plugins/marketplace.json) lists it.

## MCP server

`rbit mcp` serves the commands as MCP tools over stdio, one JSON-RPC message per line. It
answers the stateless `server/discover` of protocol 2026-07-28 and the `initialize`
handshake of 2025-03-26 through 2025-11-25. Each tool call is parsed as the matching command
line and returns that command's JSON as `structuredContent`.

| Tool | Command |
| --- | --- |
| `status`, `setup`, `build`, `verify`, `diff` | The project commands |
| `repair` | `rbit repair`: drop saved preludes the build no longer needs |
| `grind` | `rbit grind` with `step`, `symbol`, `reference_object`, `family`, `limit`, `accept` |
| `tool_list`, `tool_info`, `tool_install`, `tool_remove` | The catalog commands |
| `schema` | `rbit schema project` or `catalog` |

The server is optional: build without it using `cargo install --no-default-features`.
