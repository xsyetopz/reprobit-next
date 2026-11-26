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

## Lock

`rbit setup` writes `reprobit.lock` with each tool's version and the SHA-256 of its installed
files. Commit it. Every build hashes the compiler again and refuses to run when the content
differs from the lock, so two machines that both pass `rbit verify` ran the same bytes.
