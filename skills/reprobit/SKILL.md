---
name: reprobit
description: Rebuild game and legacy binaries byte for byte with their original compilers using the `rbit` CLI. Use for matching decompilation, reproducing an executable or object from source, installing retro compilers and reverse-engineering tools (MSVC, IDO, PS1/PS2/N64/GameCube/Wii GCC and MWCC, objdiff, splat, dtk, m2c, Ghidra, Flash's ffdec and Ruffle), or finding why one function's bytes differ.
license: LGPL-3.0-only
compatibility: Needs `rbit` on PATH (`cargo install --git https://github.com/xsyetopz/reprobit-next reprobit`). Win32 compilers need Wine, wibo, or Docker; Linux compilers off Linux need Docker or OrbStack.
allowed-tools: Bash(rbit:*)
---

# ReproBit

`rbit` installs pinned compilers and tools into a per-user home, builds a project from declared
inputs only, and compares every target with its original. Pass `--json` to every command; the
exit code is 0 for success, 1 for a negative result such as a mismatch, 2 for an error.

## Loop

1. `rbit --json status [dir]` reads the project and returns `next`, the one thing to do.
2. `rbit --json setup` installs the project's compiler and tools and writes `reprobit.lock`.
3. `rbit --json build` rebuilds only steps whose inputs changed.
4. `rbit --json diff` lists the first differing byte runs, with section and RVA.
5. `rbit --json verify` rebuilds from scratch and writes the trust report. Only `verify`
   certifies a match; quote its `identical` field, never a build's.

## Finding a function's compiler state

When a function's source is right but its bytes differ, an earlier declaration often changed
the compiler's internal numbering. `rbit grind` searches declaration-only preludes, smallest
first:

```sh
rbit --json grind --step compile.main --symbol _transform --reference-object ref/main.obj
```

Add `--accept` to write the match into `reprobit.toml`, then run `verify`. A prelude emits no
code or data, so it never copies bytes from the original. For i386 objects, grind also tries
renaming `ebx`, `esi`, and `edi`, which every calling convention preserves, and may save a
`registers` rename beside the prelude when only register allocation differs. Add-ons listed in
`toolchain.tools`, such as `msvc-4x` for MSVC 4.x instruction order, register choice, and operand order, propose rewrites too, and grind
saves one under `rewrite` only when it makes the function match. After editing a shared header or
other source, `rbit --json repair` drops each saved prelude the build no longer needs.

To start from an existing CMake, Ninja, Meson, or Bear build, `rbit import
compile_commands.json` prints its compile steps as TOML to append to `reprobit.toml`.

## Tools

- `rbit --json tool list <text>` searches the catalog by id, tag, or platform (`ps2`, `gc`, `n64`, `x86`).
- `rbit tool run <id>[:command] -- <args>` installs on first use, then runs the tool.
- `rbit --json tool remove <id>` or `rbit tool remove --all` deletes what was installed.
- decomp.me's compilers are `decompme/<platform>/<name>`, such as `decompme/ps2/ee-gcc2.96`.

## Rules

- Never read, copy, or embed bytes from a reference into a step. `rbit` refuses steps that
  declare a reference as input; keep it that way.
- Never commit references, compilers, or `.reprobit/`. Commit `reprobit.toml` and `reprobit.lock`.
- `rbit schema project` prints the JSON Schema for `reprobit.toml`.
