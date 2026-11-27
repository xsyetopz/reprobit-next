# Agents

`rbit` is the interface. Every surface below runs the same commands and returns the same
JSON, so an agent's evidence is what a person would see.

## CLI

`rbit --json <command>` prints one JSON value on stdout; progress goes to stderr, and `-q`
silences it. Exit codes: 0 success, 1 a negative result (a mismatch, a status with work
left, a grind with no match), 2 an error, whose JSON is `{"error": "…"}`.

## Skills

The skills live in [`plugins/reprobit/skills/`](../plugins/reprobit/skills) and use only
portable Agent Skills frontmatter, so any harness that reads Agent Skills can load them. Each
description stays within 200 characters.

| Skill | Job |
| --- | --- |
| `reprobit` | The loop: `status`, `setup`, `build`, `diff`, `grind`, `verify`, and the tool catalog |
| `reprobit-match` | One function to a match: score with `scripts/fnscore.py`, one change per attempt, budgets |
| `reprobit-decompile` | First source for an unclaimed function from an m2c or Ghidra draft |
| `reprobit-coordinate` | A lead with workers: queue, briefs, caps, hand-back checks, AI-contribution rules |
| `reprobit-disk` | `rbit space` and `rbit gc` on small disks |

`fnscore.py` prints `MATCH` (exit 0), `DIFF <symbol> N of M instructions differ` (exit 1),
an error (exit 2), or, with `--source FILE`, `REPEAT` (exit 3) when that exact source was
already scored. The match skill switches strategy after 3 attempts without a lower `N`, stops
after 8, and caps one function at 25 builds per job.

## Plugins

A plugin installs as one folder, so each harness plugin carries a copy of the skills.
`just plugins-sync` refreshes the copies from `plugins/reprobit/skills/`, and
`cargo test -p reprobit --test plugins` fails when a copy drifts, a description passes 200
characters, or a manifest names a missing path. Install one plugin per harness: the universal
and the harness plugin carry the same skills.

| Folder | Harness | Adds |
| --- | --- | --- |
| [`plugins/reprobit`](../plugins/reprobit) | Any | Skills, model-free agents, `agents.example.toml` |
| [`plugins/reprobit-claude`](../plugins/reprobit-claude) | Claude Code | Agents pinned to Haiku, Sonnet and Opus 5.5; `reprobit-team` skill; git guard hook |
| [`plugins/reprobit-codex`](../plugins/reprobit-codex) | Codex | `reprobit-team` skill that authorizes spawns; agent role files |
| [`plugins/reprobit-antigravity`](../plugins/reprobit-antigravity) | Antigravity | Gemini 4 agents, rules, `reprobit-team` skill, git guard hook |
| [`plugins/reprobit-pi`](../plugins/reprobit-pi) | pi | `rbit` tool, git and `reference/` guard, `/rbit-match` and `/rbit-disk` prompts |

Every guard blocks `git add` of a path under `reference/` or `.reprobit/` and of disc images
and ROM dumps (`.iso`, `.cue`, `.chd`, `.gcm`, `.rvz`, `.wia`, `.wbfs`, `.ciso`, `.gcz`,
`.mdf`, `.nsp`, `.xci`, `.z64`, `.n64`, `.v64`). It reads only the command text, so
`git add -A` and `git add .` rely on `.gitignore`; the Claude Code hook also asks git what
those would stage.

Nothing in the plugins needs an API key: each runs on the harness's own sign-in, which
includes the $20 plans (Claude Pro, ChatGPT Plus, Google AI Pro).

### Claude Code

```sh
claude plugin marketplace add xsyetopz/reprobit-next
claude plugin install reprobit-agents@reprobit
```

| Agent | Model | Effort | Job |
| --- | --- | --- | --- |
| `reprobit-agents:reprobit-scout` | Haiku 5.5 | high | Read-only triage; compacts before 100,000 tokens |
| `reprobit-agents:reprobit-matcher` | Sonnet 5.5 | medium | 1 to 4 functions in one file |
| `reprobit-agents:reprobit-writer` | Sonnet 5.5 | medium | First source for unclaimed functions |
| `reprobit-agents:reprobit-heavy` | Opus 5.5 | high | Over 0x400 bytes, or a matcher left it unchanged |

On Claude Pro the lead runs 1 worker at a time; Max plans raise it to 3. The scout's
`autoCompactWindow: 100000` needs Claude Code v2.1.296 or later.

The agents name full model IDs (`claude-haiku-5-5`, `claude-sonnet-5-5`, `claude-opus-5-5`)
rather than aliases. Claude Code maps an ID to each provider's own name, while the `haiku`
and `sonnet` aliases resolve to the 4.5 models on Bedrock, Vertex, Foundry and Mantle.
Matcher and writer run at medium, the 5.5 models' default. The catalog's effort index for
Sonnet 5.5 is 1 at high, 1.87 at xhigh and 5.5 at max, so the long matching loops stay at
medium.

### Codex

```sh
codex plugin marketplace add xsyetopz/reprobit-next
codex plugin add reprobit-codex@reprobit
cp plugins/reprobit-codex/agents/*.toml ~/.codex/agents/
```

Codex plugins carry skills and MCP servers but no agents, so the role files ship beside the
plugin; [`.codex/agents/`](../.codex/agents) holds the same files for this repository once
the project is trusted. The `reprobit-team` skill authorizes the lead to spawn workers and to
set `model` and `reasoning_effort`, which Codex's base prompt otherwise forbids, and spawns
with `fork_turns="none"` and a self-contained brief.

| Role | Model | Effort | Job |
| --- | --- | --- | --- |
| `reprobit_scout` | `gpt-6-luna` | high | Read-only triage |
| `reprobit_matcher` | the lead's | high | 1 to 4 functions in one file |
| `reprobit_writer` | the lead's | high | First source for unclaimed functions |
| `reprobit_heavy` | the lead's | xhigh | Over 0x400 bytes, or stalled |
| `reprobit_visual` | `gpt-6.1-terra` | high | Screenshots and rendered output, where bytes cannot be compared |

Run the lead on `gpt-6.2-sol` at high effort so the matching roles run on Sol. The plugin's
[`profiles/`](../plugins/reprobit-codex/profiles) holds one Codex profile per lead model:
copy them to `~/.codex` and start with `codex --profile reprobit-sol`, `reprobit-luna` for a
plan without Sol, or `reprobit-astra` for a session of functions Sol left stalled. Those roles name no model
because a ChatGPT plan serves only some models (`codex debug models` lists them) and a role
file's `model` wins over the spawn's: a ChatGPT account without Sol failed every spawn of a
role pinned to it, and the same role without a pin ran on the lead's `gpt-6-luna` at the
role's effort. Terra is priced high for coding work, so it stays out of the default loop.
`gpt-6.2-astra`, the most capable model at four times Sol's price, is the last tier for one
function: the lead spawns `reprobit_heavy` with `model="gpt-6.2-astra"` after a heavy job on
its own model ended at the `DIFF N` it started from.
ChatGPT Plus runs 1 worker at a time; Pro raises it to 3.

### Antigravity

```sh
agy plugin install <clone>/plugins/reprobit-antigravity
```

To load it for one project only, copy the folder to `.agents/plugins/reprobit-antigravity/`
in that project; Antigravity discovers plugins there, rules and hooks included.

| Agent | Tier | `/swarm` model | Job |
| --- | --- | --- | --- |
| lead | inherit | `gemini-4-hydrogen-high`; `gemini-4-neon-medium` for multi-module work | Queue, headers, verify, commits |
| `reprobit-scout` | `flash_lite` | `gemini-4-hydrogen-low` | Read-only triage |
| `reprobit-matcher` | `flash` | `gemini-4-hydrogen-high` | 1 to 4 functions in one file |
| `reprobit-writer` | `flash` | `gemini-4-hydrogen-high` | First source for unclaimed functions |
| `reprobit-heavy` | `pro` | `gemini-4-carbon-high` | Over 0x400 bytes, or stalled |

Concurrent authors use `workspace: "share"`. Google AI Pro runs 1 worker at a time and
keeps the lead on Hydrogen High; Ultra raises it to 3. Argon and Gemini 3.8 Flash are not
used.

The plugin's MCP server registers as `reprobit-antigravity_reprobit`. Headless `agy -p`
cannot ask before an MCP call, so a scripted run allows it in settings:
`--settings '{"permissions":{"allow":["mcp(reprobit-antigravity_reprobit/*)"]}}'`.
Antigravity starts the server outside the project, so pass `dir` as an absolute path.

### pi

```sh
pi install <clone>/plugins/reprobit-pi
```

pi has no subagents. The package registers an `rbit` tool, which runs `rbit --json` and keeps
the last 50 KB or 2000 lines of output, writing the rest to a temporary file.

### Other harnesses

The universal plugin's agents name no model, so they run on the lead's. For harnesses that
pick a model per spawn, such as ones serving Kimi, GLM, or DeepSeek, copy
[`agents.example.toml`](../plugins/reprobit/agents.example.toml) to `reprobit.agents.toml` in
the project, or to `agents.toml` in the directory `rbit tool home` prints, and fill in the
model ids your harness lists. The `reprobit-coordinate` skill reads it before each spawn.

## MCP server

`rbit mcp` serves the commands as MCP tools over stdio, one JSON-RPC message per line. It
answers the stateless `server/discover` of protocol 2026-07-28 and the `initialize`
handshake of 2025-03-26 through 2025-11-25. Each tool call is parsed as the matching command
line and returns that command's JSON as `structuredContent`.

| Tool | Command |
| --- | --- |
| `status`, `setup`, `build`, `verify`, `diff` | The project commands |
| `repair` | `rbit repair`: drop saved preludes the build no longer needs |
| `space`, `gc` | `rbit space` and `rbit gc` with `dir`, `keep_days`, and for `gc` `images` and `dry_run` |
| `grind` | `rbit grind` with `step`, `symbol`, `reference_object`, `family`, `limit`, `accept` |
| `tool_list`, `tool_info`, `tool_install`, `tool_remove` | The catalog commands |
| `schema` | `rbit schema project` or `catalog` |

The server is optional: build without it using `cargo install --no-default-features`.
