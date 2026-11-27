---
name: reprobit-team
description: Run ReproBit work on Claude Code with the plugin agents. Pick the agent per job, size concurrency to the plan, and write self-contained briefs. Use when delegating match or decompile jobs.
license: LGPL-3.0-only
---

# Lead a ReproBit team on Claude Code

Read `reprobit-coordinate` first for the queue, caps and hand-back checks. This skill maps
those rules onto the agents this plugin ships.

## Agents

| Agent | Model | Job |
| --- | --- | --- |
| `reprobit-agents:reprobit-scout` | `claude-haiku-5-5` (high) | Read-only triage: what differs, worst first. Run it before queueing. |
| `reprobit-agents:reprobit-matcher` | `claude-sonnet-5-5` (medium) | 1 to 4 drafted or near-matching functions under 0x400 bytes in one file. |
| `reprobit-agents:reprobit-writer` | `claude-sonnet-5-5` (medium) | First source for functions no file claims yet. |
| `reprobit-agents:reprobit-heavy` | `claude-opus-5-5` (high) | A function over 0x400 bytes, or one a matcher left at the same count. |

## Plans

- Claude Pro ($20): run 1 worker at a time. Use Haiku 5.5 scouts and Sonnet 5.5 matchers and writers.
  Spend Opus 5.5 only on a function over 0x400 bytes or one that stalled after a Sonnet job.
  Keep your own context small.
- Max plans: raise to 3 concurrent workers. The rest of the rules stay the same.

## Briefs

- Write each brief so it stands alone: file, addresses, symbols, compile step, scorer command
  (`fnscore.py` from `reprobit-match`), time cap, and report format. A worker does not see
  your conversation.
- Give each worker one file. No two workers share a file.
- Header and shared-type changes stay with you. A worker reports the change it needs; you
  make it between jobs, never in parallel.
- Workers never commit. Check each hand-back, run `rbit --json verify`, then commit one
  small slice with explicit paths.

## Guard

The plugin's PreToolUse hook blocks `git add` and `git commit -a` that would stage
`reference/`, `.reprobit/`, or disc images. If it blocks a command, fix the staging; do not
look for a way around it.
