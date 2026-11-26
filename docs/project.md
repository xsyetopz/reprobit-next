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
environment, prelude, inputs, headers, and runner are unchanged. `rbit verify` never uses that cache.

### Importing steps

`rbit import compile_commands.json` prints one compile step per entry of a compilation
database, as CMake (`-DCMAKE_EXPORT_COMPILE_COMMANDS=ON`), Ninja (`ninja -t compdb`), Meson,
and Bear write it. Sources become `${SOURCE}/…` and objects become build-root file names, so
the steps hold no machine's paths. Append them with `rbit import compile_commands.json >>
reprobit.toml`, then add the link step, which compilation databases do not record.

## Targets

`reference` is the original binary. It belongs to the project's owner and is never committed
or redistributed; ignore it in `.gitignore`. `sha256` catches a wrong original before any
comparison. `normalize` names metadata fields to clear on both sides before comparing:

| Name | Clears |
| --- | --- |
| `pe-timestamps` | The PE file header, export directory, debug directory, and resource directory time stamps |

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

## Lock

`rbit setup` writes `reprobit.lock` with each tool's version and the SHA-256 of its installed
files. Commit it. Every build hashes the compiler again and refuses to run when the content
differs from the lock, so two machines that both pass `rbit verify` ran the same bytes.
