# ReproBit

![ReproBit: a dot-matrix R assembled from individual bits](docs/assets/reprobit-mark.svg)

**An umbrella for game decompilation, recompilation, and reconstruction projects: one CLI
that fetches the original compilers and the community's tools, rebuilds a binary byte for
byte, and shows why the result can be trusted.**

`rbit` is built for AI agents first and people second. Every command answers in JSON with
`--json`, names the next step when something is missing, and exits 0, 1 (a negative result,
such as a mismatch), or 2 (an error).

- **One catalog, nothing vendored.** [`catalog/tools.toml`](catalog/tools.toml) names 36
  tools, Flash's ffdec and Ruffle among them, and the 230 compilers decomp.me serves for 17 platforms: PS1, PS2, PSP, N64,
  GameCube and Wii, DS, 3DS, Wii U, GBA, Saturn, Dreamcast, Xbox 360, Switch, Win32, and
  more. Each download is checked against a digest, installed per user, and removable by
  `rbit`; `reprobit.lock` pins what a project used.
  Compilers that are not free to redistribute, such as the original Xbox's XDK, join through
  a team catalog in [`REPROBIT_CATALOGS`](docs/catalog.md#your-own-catalog) and run under
  the same Win32 runners.
- **Any host.** Apple Silicon and Intel macOS, x86-64 Linux, and Windows. Win32 compilers
  run under Wine, wibo, or a Wine container; Linux compilers run natively or in Docker or
  OrbStack; macOS builds run natively where they exist.
- **Honest matches.** A step sees only its declared inputs, never the original binary.
  `rbit verify` rebuilds from scratch and writes a report that names the compiler's content
  digest and every input and output.
- **Agent surfaces.** A [skill](skills/reprobit/SKILL.md), a Claude Code plugin, a Codex
  plugin, and an optional MCP server (`rbit mcp`).

## Install

```sh
cargo install --locked --git https://github.com/xsyetopz/reprobit-next reprobit
```

Release binaries for each host are attached to version tags. Tools install under
`rbit tool home`: `~/Library/Application Support/ReproBit` on macOS,
`$XDG_DATA_HOME/reprobit` on Linux, `%LOCALAPPDATA%\ReproBit` on Windows, or
`$REPROBIT_HOME`. `rbit tool remove --all` deletes everything there.

In Claude Code, the plugin brings the skill and the MCP server:

```text
/plugin install reprobit --marketplace xsyetopz/reprobit-next
```

## Use

```sh
rbit tool list ps2                     # search the catalog
rbit tool run objdiff-cli -- --version # install on first use, then run
rbit init --compiler decompme/n64/ido7.1
rbit status                            # what is ready, and the next step
rbit setup                             # install the toolchain, write reprobit.lock
rbit build                             # incremental build
rbit diff                              # where the build differs from the original
rbit verify                            # cold build, byte comparison, trust report
rbit repair                            # after an edit, drop preludes no longer needed
rbit import compile_commands.json      # compile steps from CMake, Ninja, Meson, or Bear
```

[`examples/grind`](examples/grind) is a complete MSVC 4.2 project that starts one byte
away from its original and shows `rbit grind` finding the compiler state that closes it.

## Documentation

- [Project file](docs/project.md): `reprobit.toml` steps, targets, and preludes.
- [Catalog](docs/catalog.md): tool entries, fetch kinds, and adding your own catalog.
- [Runners](docs/runners.md): how Win32 and Linux compilers run on each host.
- [Add-ons](docs/addons.md): community rewrites per compiler, such as `msvc-4x`, and their protocol.
- [Agents](docs/agents.md): the skill, plugins, and MCP server.
- [Research notes](docs/research.md): the projects and tools this builds on.

The schemas in [`schemas/`](schemas) describe both documents for editors; `rbit schema`
prints the same JSON Schema.

## License

LGPL-3.0-only. See [NOTICE](NOTICE) for provenance. Compilers and tools keep their own
licenses; ReproBit redistributes none of them.
