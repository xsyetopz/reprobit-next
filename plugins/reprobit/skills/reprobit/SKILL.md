---
name: reprobit
description: Rebuild binaries byte for byte with their original compilers through the rbit CLI. Use to set up a matching project, build, diff, grind, verify, or install retro compilers.
license: LGPL-3.0-only
compatibility: Needs rbit on PATH. Win32 compilers need Wine, wibo, or Docker; Linux compilers off Linux need Docker or OrbStack.
---

# ReproBit

`rbit` installs pinned compilers and tools, builds a project from declared inputs only, and
compares every target with its original. Add `--json` to every command and read the JSON.

Exit codes: `0` success, `1` a negative result such as a mismatch (not a failure), `2` an
error whose JSON is `{"error": "..."}`.

## Loop

1. `rbit --json status` returns `next`: do that one thing.
2. `rbit --json setup` installs the compiler and tools and writes `reprobit.lock`.
3. `rbit --json build` rebuilds only the steps whose inputs changed.
4. `rbit --json diff` lists the first differing byte runs with section and RVA.
5. `rbit --json verify` rebuilds from scratch. Only its `identical` field certifies a match.

## Progress

Give each compile step `reference-object = "reference/obj/<file>.obj"`, the original object
the step should equal. Then:

- `rbit --json score` grades every function of every such step: `exact`, `reloc` (only
  relocation targets differ, such as a string literal's generated name), `different` with
  `matching_percent`, or `missing`. `matched_percent` is the project's progress figure.
- `rbit --json score <reference.obj> <built.obj> --symbol <name>` grades two objects directly.
- `rbit objdiff` writes `objdiff.json`, so objdiff's GUI and `objdiff-cli report generate`
  open the same units and rebuild through `rbit build --object`.

`reloc` is a match for progress only; `verify` decides whether the linked binary is identical.

## Recompilation

A target rebuilt with another compiler or for another platform cannot be byte-identical.
Declare `[[check]]` entries (a host command with `${BINARY}`, such as
`["rbit", "-q", "tool", "run", "wibo", "--", "${BINARY}"]` for a Win32 console program) and run `rbit --json behave`: each check runs on the reference
and on the artifact, and `agree` is true when the exit code and output match. Report it as
behaviour on those inputs, never as a match.

## One function

To match one function, use the `reprobit-match` skill. To write a function no source has
yet, use `reprobit-decompile`. To run several workers, use `reprobit-coordinate`.

When the source is right but the bytes differ, the compiler's internal numbering often
moved. Search declaration-only preludes:

```sh
rbit --json grind --step compile.main --symbol _transform --reference-object ref/main.obj
```

Add `--accept` to save a match into `reprobit.toml`, then run `verify`. A prelude emits no
code or data, so it never copies bytes from the original. For i386 objects grind also tries
renaming `ebx`, `esi` and `edi`, which every calling convention preserves. Add-ons listed in
`toolchain.tools`, such as `msvc-4x` for MSVC 4.x instruction and operand order, propose
rewrites, and grind saves one only when it makes the function match. After editing a shared
header, `rbit --json repair` drops each saved prelude the build no longer needs.

## Start a project

- `rbit init --compiler <id>` writes `reprobit.toml`; `rbit schema project` prints its schema.
- `rbit import compile_commands.json` prints compile steps from CMake, Ninja, Meson, or Bear.

## Tools

- `rbit --json tool list <text>` searches compilers and tools by id, tag, or platform
  (`ps2`, `gc`, `n64`, `x86`).
- `rbit tool run <id>[:command] -- <args>` installs on first use, then runs the tool.
- `rbit --json tool remove <id>` deletes an installed tool.
- decomp.me's compilers are `decompme/<platform>/<name>`, such as `decompme/ps2/ee-gcc2.96`.

## Disk

`rbit --json space` shows what ReproBit holds; `rbit gc` frees what can be fetched or
rebuilt. See the `reprobit-disk` skill.

## Rules

- Never read, copy, or embed bytes from a reference into a source or step. `rbit` refuses a
  step that declares a reference as input.
- Never commit references, compilers, ROMs, disc images, or `.reprobit/`.
- Commit `reprobit.toml` and `reprobit.lock`.
