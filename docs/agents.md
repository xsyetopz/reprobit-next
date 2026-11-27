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

## Subagents

A subagent works in its own context and returns a summary, so `rbit`'s long JSON, logs, and
disassembly stay out of the main conversation. Two roles cover the loop:

| Agent | Model | Effort | Context | Job |
| --- | --- | --- | --- | --- |
| `scout` | Haiku 5.5 | high | 100,000 tokens | Read-only triage: `status`, `diff`, `verify`, `tool list`; returns which targets, sections, and functions differ |
| `matcher` | Sonnet | medium | the model's | One function: edits its source, runs `build`, `diff`, and `grind`, and ends on `verify` |

Scouts read and never write, so several can run in parallel. Give each matcher its own
source file, because two agents editing one file conflict.

### Claude Code

The plugin ships both agents from [`agents/`](../agents) as `reprobit:scout` and
`reprobit:matcher`. The scout's `autoCompactWindow: 100000` makes it compact before its
context passes 100,000 tokens, and needs Claude Code v2.1.296 or later; Haiku 5.5 needs
v2.1.293. In the debug log, a request from the scout carries `source=agent:custom:reprobit:scout`
and an effective window of 80,000 tokens, which is 100,000 minus the room kept for output.

### Codex

Codex plugins carry skills, MCP servers, apps, and hooks, but no agents. The agents live in
[`.codex/agents/`](../.codex/agents) as `reprobit_scout` and `reprobit_matcher`. Codex loads
them in this repository once the project is trusted. To use them in other projects, copy them
into `~/.codex/agents/`.

An agent file sets `model` and `model_reasoning_effort`, but it inherits the parent session's
provider, and Codex speaks only the Responses API. The Claude model ids therefore resolve only
when the session's provider serves Claude through `/v1/responses`, such as a gateway. Codex
ignores `model_context_window` in an agent file, because it keeps only the model, effort,
instruction, and feature keys there. It reads a model's window from the provider's catalog
instead, from `model_catalog_url` or `model_catalog_json`. To limit the scout to 100,000
tokens, the catalog entry for `claude-haiku-5-5` sets `"context_window": 100000`. A catalog
replaces Codex's built-in model list, and each entry needs `base_instructions`.

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
