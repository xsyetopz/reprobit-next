# Catalog

[`catalog/tools.toml`](../catalog/tools.toml) names every tool and compiler `rbit` can
install: where it is published, how the download is checked, and which commands it
provides. `rbit` compiles it in. Nothing in it is vendored; each install downloads from the
upstream publisher into the per-user tool home.

```toml
[tools."objdiff-cli"]
kind = "tool"                  # compiler, tool, or runtime
summary = "Function-level object diffing for decompilation projects (command line)"
version = "3.8.3"
homepage = "https://github.com/encounter/objdiff"
license = "MIT OR Apache-2.0"
tags = ["diff", "x86", "mips", "ppc", "arm"]
bin = { objdiff-cli = "objdiff-cli" }
# runs defaults to host; also windows, linux-x86_64, linux-i386, or detect

[[tools."objdiff-cli".get]]
host = ["macos-aarch64"]       # empty means every host
url = "https://github.com/encounter/objdiff/releases/download/v3.8.3/objdiff-cli-macos-arm64"
sha256 = "27dbd79196394c246dde77ef0a6ac855ff6c568069171dd7db0093f39c03ee8d"
as = "objdiff-cli"             # the file name for a single-file download
```

## Fetches

Each `get` sets exactly one source. Several `get` entries unpack into one install, in order.

| Source | Checked by | Notes |
| --- | --- | --- |
| `url` | `sha256` | A zip, a tar (plain, gzip, or xz), or one file, recognized by content; `strip` drops a leading directory. Local paths and `file://` URLs work for mirrors. |
| `git` | `rev`, a full commit | Shallow fetch; `pick = { "from" = "to" }` keeps only some paths. |
| `oci` | Layer digests | Anonymous registry pull, such as `ghcr.io`; `reference` is a tag or a `sha256:` manifest digest. |
| `pypi` | pip | A private virtual environment, for Python tools such as splat and m2c. |
| `npm` | npm | A private prefix, for Node tools such as webcrack. |

After the fetches, `patch` applies byte patches whose original bytes must match first, and
`verify` checks the SHA-256 of named files. The install's receipt records the SHA-256 of
every installed file, which `reprobit.lock` pins per project.

`env` sets variables for every command of the tool, with the same `${…}` roots as steps.
`force-include` gives the arguments that force-include a prelude, with `{}` for its path.
`needs` lists commands that must already be on `PATH`, such as `java` for Ghidra.

## Collections

A collection turns one image template into many tools. `decompme` covers every compiler
decomp.me serves, as `decompme/<platform>/<name>`, generated from the
[decompme/compilers](https://github.com/decompme/compilers) `values.yaml`. Its members mix
Win32, Linux, and macOS builds, so `runs = "detect"` reads the installed executables: any
Mach-O means a native macOS build, any PE means Win32, and ELF means Linux, i386 or x86-64.
Members published per host OS are listed under `split`; macOS takes the `darwin` image when
there is one.

## Your own catalog

`REPROBIT_CATALOGS` adds catalogs, as paths or URLs separated like `PATH`. A later catalog
replaces an entry with the same id, so a team can pin a newer compiler or add a private
mirror without a ReproBit release.

```sh
REPROBIT_CATALOGS=./team-tools.toml rbit tool install our-compiler
```

`rbit schema catalog` prints the catalog's JSON Schema.
