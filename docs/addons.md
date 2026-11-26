# Add-ons

An add-on rewrites a step's object after it compiles, for differences between a compiler and
its reference that no source edit reaches. Add-ons are catalog tools of `kind = "addon"`, so
anyone can publish one in their own catalog, and a project uses it without a ReproBit
release. `rbit` itself keeps only rewrites that hold for every compiler, such as
[register renames](project.md#register-renames).

## Protocol

An add-on is a program in any language. Its command is the `bin` named after its id, and
`rbit` runs it in the project directory with one of two commands:

| Command | Does | Output |
| --- | --- | --- |
| `<addon> rewrite <object> <args>…` | Edits the object in place | Exit 0, or non-zero with the reason on stderr |
| `<addon> propose <built> <reference> <symbol>` | Suggests rewrites for one function | A JSON array of argument lists, `[]` for none |

A step names its rewrites, which run in order after any register renames:

```toml
[toolchain]
tools = ["msvc-4x"]

[[step]]
# …
rewrite = [{ addon = "msvc-4x", args = ["schedule", "--symbol", "_transform", "--order", "1,0,2,3"] }]
```

`rbit grind` asks each listed add-on to propose, applies each proposal to a copy, and saves
the first one whose function matches the reference byte for byte. `--family` limits grind to
named families; an add-on's id is its family.

## Trust

An add-on runs with the user's rights, like a compiler. Its catalog entry pins its download
by SHA-256, `reprobit.lock` pins its installed content, and the content digest joins the step's
cache key and its receipt, so `rbit verify` shows which add-on build shaped each output:

```text
rewrite msvc-4x schedule --symbol _transform --order 1,0,2,3 (sha256 9f2c…)
```

A rewrite must keep the program's behaviour; the add-on's own documentation states why each
of its rewrites does.

## Publishing

Release the add-on's binaries per host, then add an entry to a catalog the project loads
through `REPROBIT_CATALOGS` (see [Catalog](catalog.md#your-own-catalog)):

```toml
[tools."my-addon"]
kind = "addon"
summary = "What it rewrites, for which compiler"
version = "1.0.0"
homepage = "https://example.org/my-addon"
license = "MIT"
bin = { my-addon = "my-addon" }

[[tools."my-addon".get]]
host = ["macos-aarch64"]
url = "https://example.org/my-addon/releases/download/v1.0.0/my-addon-macos-arm64"
sha256 = "…"
as = "my-addon"
```

## `msvc-4x`

[`crates/reprobit-msvc-4x`](../crates/reprobit-msvc-4x) is the reference add-on, for i386 COFF objects from
MSVC 4.x. Its rewrite `schedule --symbol <name> --order <i,j,…>` writes a function's
instructions in a new order, numbered from 0 in their current order, and moves each
relocation with its instruction. It proposes the order that rebuilds the reference when both
functions hold the same instructions with the same relocations.

The order is checked before anything is written. It must list every instruction once, and
any two instructions it puts out of program order must not conflict, meaning neither writes
a location the other reads or writes. Locations are the 32-bit general registers, each flag,
each stack byte, the rest of memory, and the x87 and SSE state. A partial or conditional write
also reads its location, and so does an instruction that leaves a flag undefined.

Stack bytes are numbered by their offset from `esp` at the last barrier. An `[esp+d]` operand,
including `lea`, names its bytes rather than `esp`, so it may cross a `push`, a `pop`, or an
`add` or `sub esp` by a constant. Its displacement is rewritten so that it addresses the same
bytes from its new place: `mov edx, [esp+0xc]` moved above `sub esp, 8` becomes
`mov edx, [esp+0x14]`. A displacement that would outgrow its encoding is refused. Those
instructions that move `esp` keep their order among themselves.

An access through another register reaches the stack only through an address computed from
`esp`, such as `lea ecx, [esp+8]` or `mov ebp, esp`, or anything computed from those. Such an
access may reach any stack byte, so it never crosses a stack access when either one writes.
Every other register holds a pointer the function was given or loaded, which points above the
stack this function uses, so its accesses cross stack accesses freely. That holds while no
stack address escapes: when one is stored, pushed, or kept in a register across a call, every
access through another register may reach any stack byte.

Barriers conflict with everything, so they keep their place and their offset:

- branches, calls, returns, privileged and locked instructions;
- instructions that set `esp` other than by a constant, such as `leave`;
- branch targets inside the function;
- places other records name: relocations from other sections, COFF line numbers,
  `CodeView` 4 procedure starts, ends, and block ends, and the end of an FPO prologue.

The add-on refuses a function described by `CodeView` newer than version 4 or by DWARF, and
a function that jumps through a table.

The check is the predicate `reorderOk`, which [`proofs/`](../proofs) proves sound in Lean
(`reorderOk_sound`): a reordering it accepts computes the same machine state, provided each
instruction's footprint lists what it writes and what its writes depend on. The footprints
come from `iced-x86`'s instruction tables.

### `rename`

`rename --symbol <name> --map <i:from:to,…>` renames 32-bit general registers in chosen
instructions, numbered from 0, such as one value's web of definitions and uses, or a stretch
where two registers trade places. `0:edx:ecx,1:edx:ecx` makes instructions 0 and 1 use `ecx`
wherever they named `edx`, along with its `cx`, `cl`, or `ch` forms. It proposes the map that
rebuilds the reference when both functions hold the same instructions, which differ only in
the registers their operands name.

The map is checked before anything is written. Each renamed register is named by an operand
of its instruction, and neither side is `esp`. No two registers an instruction uses become
one. The renamed instruction encodes at the same length, keeps its relocated fields, and reads
and writes exactly the renamed registers of the original. The renamed function must then pass
the correspondence check below.

### `exchange`

`exchange --symbol <name> --at <i,j,…>` rewrites the short run that starts at each listed
instruction into the other of two forms that compute the same values:

| Run at `i` | Becomes | Clobbers |
| --- | --- | --- |
| `fld X` then `fadd Y` or `fmul Y`, of one width | `fld Y` then `fadd X` or `fmul X` | nothing |
| `cmp a, b` then `jb`, `jae`, `je`, `jne`, `jbe`, `ja`, `jl`, `jge`, `jle`, or `jg` | `cmp b, a` then the mirrored branch, such as `jg` for `jl` | every flag the compare sets but `ZF` |
| `mov r, [A]`, `cmp [B], r`, then `je` or `jne` | `mov r, [B]`, `cmp [A], r` | `r`, and every flag but `ZF` |

A compare flips its opcode's direction; `i:fields` instead swaps the `ModRM` fields of a
compare of two registers, the encoding MSVC writes for `cmp r32, r32`. Control may enter a run
only at its first instruction: a branch target, a place another section names, or a line
record inside it refuses the run. Relocated fields must keep their bytes and place. The new run
must read and write the same registers and flags as the old one, and the result must pass the
correspondence check with the run's clobbers dead, meaning nothing reads them before they are
written again. `propose` finds the sites whose new runs match the reference.

Addition and multiplication commute exactly, also after x87 rounding. The one difference is in
NaN payloads: when both operands are NaNs, which payload the result carries can depend on
which one was loaded first.

### The correspondence check

`rename` and `exchange` share one check, Lean Theorem 3 (`rename_sound`). For every
instruction it computes which new register or flag holds which original value, starting from
the original values at the function's start and at every place another section's relocation
names, and joining the paths that meet. It then requires:

- every instruction finds each input where the original's value now lives, on every path that
  reaches it;
- when the function leaves, the registers its caller sees hold the original's values: `eax`,
  `edx`, `ebx`, `esp`, `ebp`, `esi`, and `edi` at a `ret`, and every general register at a tail
  call or at the end of the code.

Calls follow the i386 calling conventions: a call reads `ecx`, `edx`, and the stack, clobbers
`eax`, `ecx`, `edx`, and the flags, and gets `ebx`, `esi`, `edi`, and `ebp` back unchanged. No
caller reads the flags a function returns with. A shift by a count that may be zero keeps the
flags it would set, so it also reads them. A function with a branch out of it other than a
tail call, or with a jump through a table, is refused. Debug records stay as compiled, so a
debugger may show a renamed value in its old register.

### Chains

`rewrite` takes several rewrites in a row and writes the object only when all of them succeed:

```text
rbit-msvc-4x rewrite f.obj exchange --symbol _f --at 2 rename --symbol _f --map 1:ecx:edx,2:ecx:edx
```

When no single rewrite rebuilds the reference, `propose` searches chains of up to three. Each
rewrite but the last may match the reference only in its instructions' forms, whatever
registers they name, so that a later `rename` finishes it.
