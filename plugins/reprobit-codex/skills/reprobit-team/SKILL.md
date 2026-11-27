---
name: reprobit-team
description: Run a ReproBit matching job as a team in Codex. Authorizes spawn_agent with a model and effort per role: luna scouts, sol workers, astra for stalled functions, terra visual checks.
license: LGPL-3.0-only
---

# ReproBit team in Codex

This skill authorizes the lead to delegate. When it is active, the lead may spawn subagents
and may set `model` and `reasoning_effort` on each spawn. Codex's base prompt forbids both
unless a skill or AGENTS.md asks for them; this skill asks. Follow `reprobit-coordinate` for
the queue, caps, and hand-back checks, and this skill for who runs each job.

## Spawn

`spawn_agent` takes `task_name` (lowercase letters, digits, underscores), `message`,
`agent_type`, `model`, `reasoning_effort`, and `fork_turns`.

- Always pass `fork_turns="none"`. A full-history fork inherits the parent model and effort
  and rejects overrides, so a per-spawn `model` needs `"none"`.
- With `fork_turns="none"` the worker sees nothing of your conversation. The `message` is a
  self-contained brief: source file, symbols, addresses, compile step, reference object,
  scorer command (`fnscore.py` of `reprobit-match`), budgets, and the report format.
- Pass `agent_type` as the role name when the user installed the role files
  (`~/.codex/agents/reprobit-*.toml`, or `.codex/agents/` in a trusted project). Without the
  files, pass `model` and `reasoning_effort` from the table.

## Roles

| Role (`agent_type`) | Model | Effort | Job |
| --- | --- | --- | --- |
| `reprobit_scout` | `gpt-6-luna` | high | Read-only triage: status, diff, verify, what differs |
| `reprobit_matcher` | the lead's | high | Match 1 to 4 functions in one existing source file |
| `reprobit_writer` | the lead's | high | First source for functions no file claims yet |
| `reprobit_heavy` | the lead's | xhigh | Functions over 0x400 bytes, or a job that stalled |
| `reprobit_visual` | `gpt-6.1-terra` | high | Screenshots and rendered output only |

Run the lead on `gpt-6.2-sol` at high effort where the plan serves it, so matchers, writers
and heavy jobs run on Sol. The three roles name no model because a ChatGPT plan serves only
some models (`codex debug models` lists them), and a role file's `model` wins over the
spawn's: a pinned model the plan lacks fails every spawn of that role. On a plan without Sol,
run the lead on `gpt-6-luna` and the same roles run on Luna.

The plugin's `profiles/` folder holds one Codex profile per lead model. Copy them to
`~/.codex` and start with `codex --profile reprobit-sol`, `reprobit-luna`, or
`reprobit-astra`.

## Astra

`gpt-6.2-astra` is the most capable model and costs four times Sol per token. It is the
last tier for one function, never the default:

- Spawn `reprobit_heavy` with `model="gpt-6.2-astra"` only after a heavy job on the lead's
  model ended at the same `DIFF N` it started from.
- One Astra worker at a time, on one function, with the same budgets as any heavy job.
- When the spawn reports that the model is not supported, the plan lacks Astra: keep the
  function as a draft and tell the user.

Start a whole session with `--profile reprobit-astra` only for a queue made of functions Sol
already left stalled.

`gpt-6.1-terra` is priced high and is not part of the default loop. Spawn `reprobit_visual`
only when bytes cannot be compared: an emulator frame, a Ruffle render, a recomp runtime.
Never use it for matching.

A worker whose result reads "Agent errored: The 'gpt-6.1-terra' model is not supported" did not
run, and its job is still open. For `reprobit_visual`, skip the visual check and tell the user.

## Work split

- One source file per worker. Write sets never overlap.
- Header and shared-type changes stay with you. A worker reports the header change it needs;
  you make it between jobs, never while workers run.
- Spawn the next job after you verify and commit a hand-back, not before.
- Close each finished agent so it stops counting toward the concurrency limit.
- Call `wait_agent` only when the next step depends on the result; do queue prep meanwhile.

## Scale

| Plan | Workers at once | Mix |
| --- | --- | --- |
| ChatGPT Plus ($20) | 1 | luna scouts; matchers on the lead's model at high |
| ChatGPT Pro | 3 | same roles, up to 3 matchers on disjoint files |

On Plus, run the scout first, then one matcher, and keep your own context small. Escalate a
function to `reprobit_heavy` after one sol job left it unchanged or when it exceeds 0x400
bytes. Nothing here needs an API key; every role runs on the signed-in ChatGPT plan.
