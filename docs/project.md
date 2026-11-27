# Project file

A project is one `reprobit.toml` beside its sources. It names the compiler, the steps that
turn sources into binaries, and the originals each binary must equal. `rbit schema project`
prints its JSON Schema; [`schemas/project.schema.json`](../schemas/project.schema.json) is
the same document for editors, and a `#:schema` comment on the first line turns on
completion in Taplo and Even Better TOML.

The schema has no version field. Fields are added only with defaults, so an older file stays
valid; a field that changes meaning gets a new name instead.

```toml
name = "grind"

[toolchain]
compiler = "msvc-4.2"          # catalog id
tools = ["objdiff-cli"]        # other catalog tools that steps call
runner = "auto"                # auto, native, wine, wibo, or docker

[[target]]
id = "program"
artifact = "grind.exe"         # relative to the build root
reference = "reference/grind.exe"
sha256 = "9c78…"               # the reference after normalization
normalize = ["pe-timestamps"]

[[step]]
id = "compile.transform"
program = "cl"                 # a compiler command, or tool:command
args = ["/nologo", "/c", '${SOURCE}\transform.cpp', '/Fo${BUILD}\obj\transform.obj']
inputs = ["source/transform.cpp"]
outputs = ["obj/transform.obj"]
reference-object = "reference/obj/transform.obj"  # the original object, for rbit score
```

## Steps

Steps run in order, each in the build directory. `inputs` are `source/<path>` for project
files or `build/<path>` for an earlier step's output; `outputs` are relative to the build
root. `rbit` copies only declared sources, plus files whose extension is in `headers`
(default `h`, `hh`, `hpp`, `hxx`, `inc`, `inl`), into the stage, so a step can never read an
original binary. A step that declares a reference as an input is refused.

`args` and `env` expand four roots: `${SOURCE}`, `${BUILD}`, `${TOOLCHAIN}`, and `${TEMP}`,
plus `${TOOLCHAIN_ROOTED}`, the toolchain path without its drive letter. How they are
spelled depends on the runner; see [runners](runners.md). Spelling them the same way on
every machine is what keeps paths a compiler embeds in its output reproducible.

`rbit build` reuses a step's outputs when its program, compiler content, arguments,
environment, prelude, register renames, inputs, headers, and runner are unchanged. `rbit verify` never uses that cache.

### Reference objects and progress

`reference-object` names the original object a compile step's object should equal, such as
`reference/obj/transform.obj`. Like a target's `reference`, it stays on the owner's machine,
and no step can read it.

`rbit score` grades every function those objects define. A function is `exact` when its
bytes and relocations equal the original's, and `reloc` when only the relocation targets or
the bytes the linker patches differ, as when a string literal gets another generated name.
It is `different` when the instructions differ, reported with the share of bytes that agree,
and `missing` when the built object lacks it. `matched_percent` counts `exact` and `reloc`,
the progress figure decompilation projects publish. A function graded `reloc` can still
leave the linked binary different, so `rbit verify` stays the only certificate.
`rbit score <reference> <built>` grades two objects outside a project.

A project-wide score records each `exact` or `reloc` function in `reprobit.matched`, one
`step<TAB>symbol` per line. A recorded function that no longer matches is marked
`regressed`, and the summary counts them. A header edit or a neighbouring change breaks a
match without touching its source, so the record turns that into a report. The file is
committed: every clone and worktree checks against it, and `rbit clean --all` keeps it.
`rbit score --regressions` passes unless a function regressed, so it can gate each commit
while unfinished functions still differ. `rbit score --forget` drops graded functions that
no longer match from the record, for a function that was removed or renamed on purpose.

`rbit objdiff` writes `objdiff.json` with one unit per step that has a reference object, so
[objdiff](https://github.com/encounter/objdiff) and `objdiff-cli report generate` read the
same pairs. objdiff rebuilds a unit with `rbit build --quiet --object <path>`, which runs the
incremental build and fails when no step writes that path.

### Importing steps

`rbit import compile_commands.json` prints one compile step per entry of a compilation
database, as CMake (`-DCMAKE_EXPORT_COMPILE_COMMANDS=ON`), Ninja (`ninja -t compdb`), Meson,
and Bear write it. Sources become `${SOURCE}/…` and objects become build-root file names, so
the steps hold no machine's paths. Append them with `rbit import compile_commands.json >>
reprobit.toml`, then add the link step, which compilation databases do not record.

## Targets

`reference` is the original binary. It belongs to the project's owner and is never committed
or redistributed; ignore it in `.gitignore`. `sha256` catches a wrong original before any
comparison. `normalize` names metadata fields to clear on both sides before comparing. Each
clears only values a linker fills from the clock or a random source, never code or data:

| Name | Clears |
| --- | --- |
| `pe-timestamps` | The PE file header, export directory, debug directory, and resource directory time stamps |
| `pe-checksum` | The PE optional header checksum, which covers the time stamps |
| `build-id` | The link's identity: the PE CodeView `RSDS` GUID and age or `NB10` signature and age, the ELF GNU build-id, and each Mach-O `LC_UUID` with the code signature that hashes it |

`stamp` writes declared values into the built artifact after the last step, so the file
itself matches rather than only comparing equal after `normalize`. Use it for a link time the
original recorded:

```toml
stamp = { pe-timestamp = 0x3f3a5c1e }
```

| Name | Writes |
| --- | --- |
| `pe-timestamp` | The PE file header time stamp |
| `pe-checksum` | The PE optional header checksum |

Each value is declared in `reprobit.toml` and listed in the trust report; nothing is read from
the reference while building.

## Preludes

A step may carry a `prelude`: declarations force-included before the source that produce no
code or data but change a compiler's internal numbering, such as register allocation or
label order. `rbit grind` searches them from the smallest up:

| Family | Fields | Emits |
| --- | --- | --- |
| `shape` | `classes`, `functions` | Classes with inline member functions |
| `pad` | `classes`, `functions` | A grid of classes with unused inline members |
| `forward` | `count` | Forward class declarations |
| `extern` | `count` | Extern object declarations |

```toml
prelude = { family = "forward", count = 1 }
```

The compiler's catalog entry supplies the force-include flag, such as `/FI` for MSVC or
`-include` for GCC.

A source edit can make a saved prelude unnecessary, such as a new declaration that shifts
the numbering the same way. `rbit repair` rebuilds each step that has a prelude without it,
and removes the prelude from `reprobit.toml` only when every target still matches. When
targets differ even with the preludes, it changes nothing and points at `rbit diff` and
`rbit grind`.

## Register renames

Compilers settle ties in register allocation differently, so an i386 function can match
except that two registers trade places. A step may rename registers in a function of its
object after it compiles, by symbol:

```toml
registers = { _transform = { esi = "edi", edi = "esi" } }
```

A rename permutes `ebx`, `esi`, and `edi`, which every i386 calling convention preserves, so
the function's behaviour cannot change: its saves and restores are renamed with its uses.
ELF objects leave `ebx` out, because position-independent code calls through the PLT with
the GOT address in it. The rename refuses a function it cannot rewrite in place: an
instruction that uses a renamed register without naming it, such as `rep movsd`, a byte
register such as `bl` whose new register has none, an encoding whose length would change, a
relocated field that would change, or a jump through a table, whose data may follow the
code.

Debug records follow the code, so debuggers and unwinders see the renamed registers:

| Format | Renamed in the function's scope |
| --- | --- |
| `CodeView` (`.debug$S`, MSVC 4.x and later, clang-cl) | Register variables, register-relative locals, live ranges, and the frame programs that say where saved registers live |
| DWARF (`.debug_info`, `.debug_loc`, `.debug_loclists`) | Location expressions, including location lists |
| Call-frame information (`.eh_frame`, `.debug_frame`) | The rules that restore saved registers |

Records shared with other functions are never edited in place: a frame program another
function also uses gets a renamed copy. A record the rename cannot express is refused, such
as a frame addressed from `ebx` or a DWARF entry value that names a renamed register.

`rbit grind` tries every rename on each candidate prelude, so it finds a prelude and a rename
together; `--family registers` alone tries renames without a new prelude.

## Rewrites

`rewrite` lists [add-on](addons.md) rewrites that run on the step's object after it compiles
and after its register renames, in order. `rbit grind` saves one when an add-on proposes it and
it makes the function match.

```toml
rewrite = [{ addon = "msvc-4x", args = ["schedule", "--symbol", "_transform", "--order", "1,0,2,3"] }]
```

## Behaviour checks

A recompiled target, built by a newer compiler or ported to another platform, differs from
its original byte for byte by design. A `[[check]]` compares what the two binaries do
instead:

```toml
[[check]]
id = "prints-usage"
target = "program"
run = ["rbit", "-q", "tool", "run", "wibo", "--", "${BINARY}", "/?"]  # ${BINARY} is the binary under test
stdin = "tests/usage.txt"           # optional project file fed to standard input
compare = ["exit", "stdout"]        # the default; stderr can be added
timeout = 60                        # seconds per run, the default
```

`rbit behave` builds incrementally, copies the reference and then the artifact under the
reference's file name into the same empty directory, runs the command in it, and compares
the exit code and the SHA-256 of each named stream. A check that times out fails. The report
shows the first 4 KiB of each stream. Agreement covers the declared inputs only, so a
certificate of identical bytes still comes from `rbit verify` alone. The command supplies
its own isolation, such as Wine or an emulator.

## Lock

`rbit setup` writes `reprobit.lock` with each tool's version and the SHA-256 of its installed
files. Commit it. Every build hashes the compiler again and refuses to run when the content
differs from the lock, so two machines that both pass `rbit verify` ran the same bytes.
