---
trigger: model_decision
description: Apply when delegating ReproBit work to subagents, choosing a Gemini 4 model or effort, or running a /swarm of matchers.
---

# ReproBit model routing (Gemini 4)

| Role | Agent | Tier | Model |
| :--- | :--- | :--- | :--- |
| Lead | main session | inherit | `gemini-4-hydrogen-high` daily; `gemini-4-neon-medium` for multi-module coordination |
| Scout | `reprobit-scout` | `flash_lite` | `gemini-4-hydrogen-low` |
| Matcher, writer | `reprobit-matcher`, `reprobit-writer` | `flash` | `gemini-4-hydrogen-high` (`gemini-4-hydrogen-xhigh` for one tricky function) |
| Heavy | `reprobit-heavy` | `pro` | `gemini-4-carbon-high` |

- Keep the whole session on one model; start on Hydrogen High. Switching mid-session forfeits prefix caching.
- Use Neon Medium only for multi-module coordination: it drove the largest cache-read drain in measured use. Say which criterion applied.
- Use Carbon High only for the hardest work: functions over 0x400 bytes, or drafts that stayed at the same `DIFF N` after a matcher job.
- Gemini 4 Argon and Gemini 3.8 Flash are not used in this plugin.
- A subagent starts with a cold prefix of about 14k tokens. Spawn one only when the job needs more than about 6 tool calls; do a small single-function match in the lead.
- Google AI Pro: one worker at a time. Google AI Ultra: up to 3 workers at once.
- Give concurrent authors `workspace: "share"` and one source file each. Scouts use `inherit`.
- Put file, symbols, compile step, scorer command and report format in each brief, and ask for a reply under 300 words.
- Load the `reprobit-team` skill for the full roster and `reprobit-coordinate` for queue and hand-back rules.
