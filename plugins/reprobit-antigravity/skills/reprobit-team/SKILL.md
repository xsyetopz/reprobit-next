---
name: reprobit-team
description: Gemini 4 roster for ReproBit matching work: lead, scout, matcher, writer, heavy with exact /swarm model ids, AI Pro one-worker guidance and Ultra scaling. Use before spawning workers.
license: LGPL-3.0-only
---

# ReproBit team on Gemini 4

Pair this skill with `reprobit-coordinate`, which owns the queue, the caps and the hand-back checks. This skill fixes the models and the quota budget.

## Roster

| Role | Agent | Tier | `/swarm` `model` | `workspace` | Job |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Lead | main session | inherit | `gemini-4-hydrogen-high`; `gemini-4-neon-medium` for multi-module coordination | inherit | Queue, briefs, headers, `rbit --json verify`, commits |
| Scout | `reprobit-scout` | `flash_lite` | `gemini-4-hydrogen-low` | inherit | Read-only `rbit status`, `diff`, `verify` triage |
| Matcher | `reprobit-matcher` | `flash` | `gemini-4-hydrogen-high` | share | 1 to 4 functions in one file |
| Writer | `reprobit-writer` | `flash` | `gemini-4-hydrogen-high` | share | First source for unclaimed functions |
| Heavy | `reprobit-heavy` | `pro` | `gemini-4-carbon-high` | share | Functions over 0x400 bytes, drafts stuck after a matcher job |

`gemini-4-hydrogen-xhigh` suits a matcher on a tricky single function. Gemini 4 Argon and Gemini 3.8 Flash are not used.

## Google AI Pro ($20)

- Run one worker at a time. The lead stays on Hydrogen High with a small context.
- Each subagent starts with a cold prefix of about 14k tokens (measured cache hit 59 to 79 percent). A job is worth a subagent when it needs more than about 6 tool calls. Do a single small match in the lead.
- Neon Medium in the lead made the largest quota drain in measured use (30M cache-read tokens over 149 calls). Choose it only for multi-module coordination.
- Choose the model before the context grows. Switching models mid-session forfeits prefix caching.
- Keep Carbon for the hardest work; use it through `reprobit-heavy` or `/boost`.

## Google AI Ultra

The same roster scales to 3 workers at once, the cap in `reprobit-coordinate`. Add workers only while the host has spare CPU cores, because each one runs compiler and scorer processes.

## Concurrent authors

- Give every concurrent author `workspace: "share"`. Keep scouts on `inherit`.
- One source file per worker. Two workers never edit the same file.
- Header and shared-type changes stay with the lead and never run in parallel. A worker reports the change it needs.
- Put the file, symbols, compile step, scorer command and report format in each brief. Cap the job at 30 minutes.
- After each hand-back the lead scores every matched function in the file and in files that include its header, then runs `rbit --json verify`. Only `identical` from `verify` certifies a match.
