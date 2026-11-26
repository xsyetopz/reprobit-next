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

[`crates/msvc-4x`](../crates/msvc-4x) is the reference add-on, for i386 COFF objects from
MSVC 4.x. Its rewrite `schedule --symbol <name> --order <i,j,…>` writes a function's
instructions in a new order, numbered from 0 in their current order, and moves each
relocation with its instruction. It proposes the order that rebuilds the reference when both
functions hold the same instructions with the same relocations.

The order is checked before anything is written. It must list every instruction once, and
any two instructions it puts out of program order must not conflict, meaning neither writes
a location the other reads or writes. Locations are the 32-bit general registers, each flag,
memory as one location, and the x87 and SSE state. A partial or conditional write also reads
its location, and so does an instruction that leaves a flag undefined. Barriers conflict with
everything, so they keep their place and their offset:

- branches, calls, returns, privileged and locked instructions;
- branch targets inside the function;
- places other records name: relocations from other sections, COFF line numbers,
  `CodeView` 4 procedure starts, ends, and block ends, and the end of an FPO prologue.

The add-on refuses a function described by `CodeView` newer than version 4 or by DWARF, and
a function that jumps through a table.

The check is the predicate `reorderOk`, which [`proofs/`](../proofs) proves sound in Lean
(`reorderOk_sound`): a reordering it accepts computes the same machine state, provided each
instruction's footprint lists what it writes and what its writes depend on. The footprints
come from `iced-x86`'s instruction tables.
