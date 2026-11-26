# Grind: find the compiler state that closes a one-byte gap

This project builds one C++ function with Microsoft Visual C++ 4.2. The source is already
right, but the rebuilt executable differs from its original by one byte: the compiler chose a
different register encoding because the original was built after other declarations had
changed its internal numbering. `rbit grind` searches declaration-only preludes until the
function's bytes match.

## Run it

The compiler is a Win32 program, so this needs Wine, Docker or OrbStack, or wibo
(`rbit tool install wibo`).

```sh
cd examples/grind
rbit setup .               # install MSVC 4.2, check reprobit.lock
sh prepare-reference.sh    # build the "original" into reference/
rbit verify .              # fails: 1 byte differs in .text
rbit grind . --step compile.transform --symbol _transform \
  --reference-object reference/reference.obj --accept
rbit verify .              # passes
```

`prepare-reference.sh` stands in for the original release: it compiles the same source with a
`shape` prelude, so the reference is generated rather than committed. `rbit status` checks it
against the pinned digest in `reprobit.toml`.

`grind` tries the current state first, then candidates from the smallest prelude up, and
stops at the first match: `forward count=1`, after two compiles. `--accept` writes it into
the step:

```toml
prelude = { family = "forward", count = 1 }
```

The prelude declares one class and emits no code or data, so the match comes from the
compiler, not from bytes of the original. `rbit verify` then rebuilds from scratch and writes
`.reprobit/reports/report.html`.

To start over, delete the `prelude` line and `.reprobit/`.
