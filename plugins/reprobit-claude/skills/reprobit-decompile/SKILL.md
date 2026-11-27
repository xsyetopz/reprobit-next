---
name: reprobit-decompile
description: Write C or C++ for a function no source claims yet, from its disassembly and an m2c draft, then match it with reprobit-match. Use for new functions in a matching decompilation.
license: LGPL-3.0-only
compatibility: Needs rbit. m2c covers MIPS, PS2 EE, PowerPC, ARM and SH-2; Ghidra or Rizin cover x86.
---

# Decompile a new function

## Steps

1. Check the address is free: no source in the project already claims it.
   In a project with a `decomp.yaml`, `rbit tool run coddog -- match <symbol> -t 0.9` lists
   similar functions; one marked `(decompiled)` is a better first draft than any decompiler.
2. Get a first draft:
   - MIPS, PS2 EE, PowerPC, ARM or SH-2: `rbit tool run m2c -- -t <target> -f <symbol> --context <ctx.c> <asm file>`,
     where `<target>` is such as `mips-ido-c` (N64), `mipsee-gcc-c` (PS2), `ppc-mwcc-c++` (GameCube, Wii) or `arm` (GBA), and `<ctx.c>` is
     the project's preprocessed headers.
   - x86, or where m2c fails: print Ghidra's C for the symbol with this skill's script, in an
     empty scratch directory as the Ghidra project:
     `rbit tool run ghidra-ghydra:headless -- <scratch dir> draft -import <binary>
     -scriptPath <this skill>/scripts -postScript ReprobitDecompile.java <symbol> -deleteProject`.
     The draft follows the `DRAFT <symbol> <address>` line; `DRAFT-NONE` means the symbol
     name differs, so list names by running it without a symbol. Or read the disassembly from
     `rbit tool run rizin`.
   - Interactive Ghidra, where names and types should persist between drafts:
     - Import once headless, with no `-deleteProject`:
       `rbit tool run ghidra-ghydra:headless -- <project dir> <name> -import <binary>`.
     - Open it: `rbit tool run ghidra-ghydra:ghidra -- <project dir>/<name>.gpr`. The
       GhydraMCP plugin starts with the project window, on port 8192.
     - Ghidra's first start shows its user agreement. Accepting it is the user's step.
     - `rbit tool run ghydra:ghydra -- project open-file --path /<binary name>` opens a
       CodeBrowser. Then `ghydra --json instances list` shows the file on port 8192.
     - `ghydra --json functions decompile --name <symbol>` prints the draft. `functions rename`,
       `functions set-signature` and `structs` carry names and types back for the next draft.
     - Point `GHIDRA_JAVA_OPTIONS=-Dapplication.settingsdir=<dir>` at a scratch directory so a
       throwaway session leaves the user's Ghidra settings alone.
3. Fix the signature from the mangled name and the assembly:
   - The mangled name gives the parameter types and the class.
   - The return type comes from the return register written before the return.
4. Write the function in the file its debug or assert strings name, in address order among
   the functions already there.
5. Access fields through typed struct members, never through casts at fixed offsets such as
   `*(int *)((char *)p + 0x1E0)`. Name an unknown member by its offset (`m_unk1E0`) and a gap
   as `char _pad1E4[n]`, until a string, assert, or call reveals the name. Keep globals under
   their real symbol names.
6. Mark it as a draft, build, and hand it to the `reprobit-match` skill.

## Size limits

- Up to 0x400 bytes: one pass, then match.
- Over 0x400 bytes: split it into case-sized parts (one `switch` case or one loop at a time)
  and match each part's instructions before writing the next.

## Rules

- The draft is a start, not the answer: m2c and Ghidra output rarely compiles to the same
  bytes. Expect to rewrite expressions, types, and loop shapes.
- Never paste the original's instructions as inline assembly for code the language can say.
- When the build breaks, the file breaks for everyone. Keep every edit compiling.
