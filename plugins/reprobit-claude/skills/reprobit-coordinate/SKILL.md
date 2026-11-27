---
name: reprobit-coordinate
description: Lead a matching decompilation with workers. Queue small jobs first, give each worker one file, cap its time and function size, then verify and commit each hand-back.
license: LGPL-3.0-only
---

# Coordinate matching work

These rules come from a PS2 decompilation run by agents. Draft jobs of 4 low-difference
functions in one file took 10 to 15 minutes and matched most of them. Whole-function writes
of large functions took 2 to 4.5 hours and matched none, because nothing capped their time.

## Queue

1. List unmatched functions with their size and, for drafts, their last `DIFF N`.
2. Fill the queue with drafts that have the lowest `N` first, then small free functions.
3. A job is one source file and 1 to 4 functions in it.
4. Functions over 0x400 bytes go to the strongest model you have, or get split into parts.
5. Drafts that stayed at the same `N` after a job go to the permuter
   (`rbit tool run decomp-permuter`) instead of another worker.
6. Before writing a function, look for a matched one with the same instructions apart from
   addresses (a twin). Copy its source and score; twins are the cheapest matches.

## Workers

- At most 3 workers at once. Each owns only the files its job names.
- Give each worker a self-contained brief: file, addresses, symbols, the compile step, the
  scorer command, and the report format. Do not rely on it seeing your conversation.
- Cap a worker at 30 minutes and about 100,000 tokens of context. When the cap passes, keep
  its best compiling draft with a note and close the job.
- Header or shared-type changes stay with you, never in parallel.

## Models

A harness that names no model for a worker runs it on the lead's model. When the user keeps a
role map, read it and pass its model with each spawn:

1. `reprobit.agents.toml` in the project, then
2. `agents.toml` in the ReproBit home, the directory `rbit tool home` prints.

Keys are roles (`scout`, `matcher`, `writer`, `heavy`) with `model` and `effort`. A role the
file leaves out runs on the lead's model.

## Each hand-back

1. Check the worker's file: compiles, no stray edits, line count sane.
2. Run `rbit --json score --regressions` after the build. It exits 1 when a function that
   matched before stopped matching, which a header edit does silently in any includer, and
   marks each one `regressed`. Fix those before taking the next job. Commit
   `reprobit.matched` with the slice, so every worker checks against the same record.
3. Run the full build and `rbit --json verify`.
4. Commit one small slice with explicit paths. Add an `Assisted-by:` trailer naming the agent,
   because many projects require AI-written code to be labelled.
5. Record any new source idiom the worker found in the project's idiom notes.
6. Take the next job.

## Upstream

Before proposing a pull request to a community project, read its contribution rules. Some
decompilations refuse AI-written code; then keep the work local and tell the user.

## Quota

- On a $20 plan, run 1 worker at a time on the cheaper model and keep the lead's own context
  small; on higher plans raise to 3.
- Workers on a cheap model handle drafts with low `N` and functions under 0x400 bytes.
  Escalate a function to the strong model after one cheap job left it unchanged.

## Progress

Report matched functions and bytes against the total, game code apart from library code
that came from original library objects.

- `rbit --json locate <binary> <archive or object>...` finds each library function in the
  binary with relocated fields masked. Each one is `found` at an address, `ambiguous`, or
  `absent`, and a member is `linked` when all its functions are found.
- Take the functions of linked members off the decompilation queue: link those objects
  instead.
- An `ambiguous` function shares its bytes with another place or member, as a `w` variant
  does with its base function. Confirm it from a call site, or decompile it.
