# Research notes

ReproBit stands on the community's tools rather than replacing them. These notes record what
each source contributed, so a later change can check the reasoning against the original.

## Sibling projects

| Project | Taken from it |
| --- | --- |
| [foxtacles/isle](https://github.com/foxtacles/isle) | The byte-identity method for MSVC 4.2: fixed DOS logical paths, the C2 patch, timestamp normalization, and declaration-only preludes. ReproBit began as an extraction of this tooling. |
| xsyetopz/GodHand | An i386 Debian image with Wine runs 32-bit Win32 compilers on any Docker host, Apple Silicon included through emulation; `reprobit-wine:1` follows it. |
| xsyetopz/OpenGNG | Sandboxed agents often cannot reach the Docker socket. OpenGNG relays builds through a spool directory that a host daemon watches; ReproBit instead reports the runner it chose in `rbit status`, so an agent learns early that it needs Docker access. |
| xsyetopz/OpenTLOS | A GameCube project on the dtk-template stack: `dtk`, `objdiff-cli`, `wibo` for Metrowerks compilers, and `ninja`. All four are in the catalog. |
| xsyetopz/OpenSAB | A clean-room Haxe and OpenFL rewrite of a Flash game, with the SWF as a behaviour reference only. Its pipeline is a catalog pattern for Flash: deobfuscate AVM1 bytecode first, because obfuscated control flow defeats decompilers; export scripts, sounds, and shapes with JPEXS ffdec (`ffdec-cli.jar -export script`, `-dumpSWF`); render reference frames in Ruffle and the converted art in resvg and compare them pixel by pixel; and keep converted art in Git LFS. ffdec and Ruffle are in the catalog. Its lesson for verification: when two non-reference renderers disagree, such as OpenFL and ffdec on a glow filter, neither settles the question; only the original runtime does. |

## From the Python version

ReproBit 0.2 was a Python package built around MSVC 4.2. The Rust version keeps what serves
every platform and drops what only steered that one compiler:

| Python feature | Rust |
| --- | --- |
| Declaration discovery and grind | `rbit grind`, with the same prelude families |
| Repair after a shared-header edit | `rbit repair`: a saved prelude goes only when every target still matches |
| CMake import through the CMake file API | `rbit import compile_commands.json`, which also reads Ninja, Meson, and Bear output |
| PE timestamp and NB10 debug identity normalization | `normalize`: `pe-timestamps`, `pe-checksum`, and `build-id`, which also covers PE `RSDS`, ELF build-ids, and Mach-O UUIDs |
| Content-pinned toolchains and cold verification | `reprobit.lock` and `rbit verify` |
| Versioned JSON records (`project-v3`, `report-v2`, and others) | `reprobit.toml` and two unversioned JSON Schemas generated from the types |
| COFF projection, donors, mosaics, and composition recipes with proof certificates | Not ported. They rewrote MSVC 4.2 objects after compilation, so a match depended on ReproBit's rewriter as well as the compiler. A match now comes from the compiler, a prelude, or a pinned catalog patch, and any further processing is an ordinary step that runs a catalog tool such as objdiff or dtk. |
| PDB 2.00 SmallMSF canonicalization for debug companions | Not ported; only MSVC 4.x PDBs needed it, and objdiff compares functions without a PDB. |

## Compilers

- **decomp.me** publishes every compiler it serves as an OCI image at
  `ghcr.io/decompme/compilers/<platform>/<id>`, with files under
  `/compilers/<platform>/<id>`. Image names are lowercase and spell `+` as `plus`. Compilers
  with host-specific builds add `/linux` or `/darwin`. The list comes from
  [`values.yaml`](https://github.com/decompme/compilers/blob/main/values.yaml).
- **archaic-msvc** hosts MSVC 4.x and 5.0 trees as git repositories, which the catalog pins
  by commit.
- **decompals** publishes [wibo](https://github.com/decompals/wibo) and the PSX GCC builds
  ([old-gcc](https://github.com/decompals/old-gcc)) used by PS1 projects;
  [ido-static-recomp](https://github.com/ethteck/ido-static-recomp) runs IRIX IDO natively.

## Tools

| Tool | Role |
| --- | --- |
| [objdiff](https://github.com/encounter/objdiff) | Function-level diffs of object files; the standard for matching decompilation. |
| [decomp-toolkit](https://github.com/encounter/decomp-toolkit) | GameCube and Wii DOL and REL analysis and splitting. |
| [splat](https://github.com/ethteck/splat) and spimdisasm | Splitting N64, PS1, and PS2 binaries into assembly and data. |
| [m2c](https://github.com/matt-kempster/m2c) | MIPS and PowerPC assembly to C. |
| [reccmp](https://github.com/isledecomp/reccmp) | Function matching for Win32 PE decompilations. |
| decomp-permuter, asm-differ | Random source permutation and assembly diffing. |
| Ghidra, Rizin | Full disassemblers and decompilers. |
| Apktool, webcrack, wakaru | Android APK and JavaScript bundle reconstruction. |
| [JPEXS ffdec](https://github.com/jindrapetrik/jpexs-decompiler), [Ruffle](https://github.com/ruffle-rs/ruffle) | Flash: SWF decompiling and export, and a Flash Player emulator as the behaviour reference. |
| [rea](https://github.com/morluto/rea) | An agent-driven reverse-engineering MCP server. |

## Reference material

- [decomp.me](https://decomp.me) and the [decompilation wiki](https://decompilation.wiki) for
  per-platform compiler flags and matching techniques.
- [decomp.dev](https://decomp.dev) tracked project progress but is closed source;
  [encounter/frogress](https://github.com/encounter/frogress) is an open alternative.
- [cli-guidelines](https://clig.dev): results on stdout, progress on stderr, `--json`,
  `-q`, and errors that say what to do next.
- Agent skills for decompilation, such as Game-Decompilation-Skill-For-Claude, keep to a
  short loop, the commands the agent may run, and the rules it must not break; the
  ReproBit skill follows that shape.
- [radare2](https://github.com/radareorg/radare2) and Cutter,
  [mytechnotalent/Reverse-Engineering](https://github.com/mytechnotalent/Reverse-Engineering),
  and [sindresorhus/awesome](https://github.com/sindresorhus/awesome) were surveyed for
  tools; the catalog carries Rizin, the radare2 fork that Cutter is built on.
