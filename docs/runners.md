# Runners

A compiler's catalog entry says where its executables run (`runs`), and the project's
`[toolchain] runner` says how. `auto`, the default, picks the first that works on this host:

| Compiler runs on | Windows | macOS | Linux x86-64 |
| --- | --- | --- | --- |
| `host` | native | native | native |
| `windows` (Win32) | native, `subst` drive | Wine, then wibo, then Docker | Wine, then wibo, then Docker |
| `linux-x86_64`, `linux-i386` | Docker | Docker | native |

`rbit status` names the runner it chose; `runner = "native"`, `"wine"`, `"wibo"`, or
`"docker"` forces one. `[toolchain] runs` overrides the catalog for one project.

## Paths the compiler sees

Each runner shows the compiler the same stage: the declared sources, the build directory,
the toolchain, and a temporary directory.

- **Wine and Windows** see a private drive: `R:\source`, `R:\build`, `R:\toolchain`, and
  `R:\Users\reprobit\AppData\Local\Temp` unless `[paths]` says otherwise. Wine runs in a
  private prefix under `.reprobit/wine` whose only drives are `c:` and the stage, so the
  host filesystem is out of reach. On Windows, `subst` maps the stage to the drive.
- **Docker** mounts the stage at `/r` and the toolchain read-only below it.
- **Native and wibo** pass paths relative to the build directory, such as
  `../source/main.c`, which are the same on every machine and never look like a `/x` switch
  to a DOS-style command line.

## Docker and OrbStack

Docker Desktop, OrbStack, Colima, and Podman all work; `rbit` calls `docker`, or `podman`
when there is no `docker`. Linux compilers run in `debian:bookworm-slim`, pinned by digest,
on `linux/amd64` or `linux/386`. Win32 compilers run in a local `reprobit-wine:1` i386 image
that `rbit` builds once from a Dockerfile it carries. On Apple Silicon both run under
emulation: OrbStack and Docker Desktop provide it, and the first build pulls or builds the
image.

## Wine and wibo

- **Wine**: `brew install --cask wine-stable` on macOS, or `apt install wine` on Linux.
- **wibo**: `rbit tool install wibo` installs [wibo](https://github.com/decompals/wibo), a
  small Win32 loader for command-line compilers that runs on Linux and macOS without Wine.
  It runs MSVC 4.2 and the Metrowerks compilers; GUI programs and installers need Wine.

## Host notes

- **Linux x86-64**: i386 compilers need 32-bit libraries, such as
  `dpkg --add-architecture i386 && apt install libc6:i386`, or `runner = "docker"`.
- **Intel macOS 13 and later** and **Apple Silicon**: macOS builds in the catalog, such as
  decomp.me's IDO and EGCS for N64, run natively; Linux and Win32 compilers need Docker,
  OrbStack, Wine, or wibo.
- **Windows**: Win32 compilers run directly. Linux compilers need Docker Desktop with the
  WSL 2 backend.
