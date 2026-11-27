# Metrowerks CodeWarrior PS2 idioms

Each pair was proven on `mwccps2` 3.0.x at `-O4` in a matching decompilation of a PS2 game
(xsyetopz/OpenGNG, 2,282 of 3,018 functions matched). Write the source form on the right of
each item.

## Branches

- An if/else that assigns one variable compiles as a ternary; the ternary changes delay-slot
  scheduling.
- An early `return` matches where a wrapping `if` does not. In a retry loop, write
  `return 0;`, not `r = 0; goto done;`.
- `if (f)` on a float flips the `c.eq.s` operand order against `if (f != 0.0f)`.
- Range tests: `(unsigned)(x - a) <= n` and `< n + 1` compile differently; the original's
  `sltiu` immediate tells which.
- An unsigned compare put in `$at` comes from `x > N - 1U`.

## Loops

- A loop entered with `b` to a test at its bottom, where yours tests at the top, is a
  goto-entered loop: `i = 0; goto test; loop: body; i++; test: if (i < n) goto loop;`.
  `for`, `while`, and `do`/`while` all put the test at the top.
- Loop-advance tails: `p = (++i < n) ? arr[i] : 0;`. The same `if`/`else` leaves an extra
  `b` and `nop`.
- When the element type differs from the storage type, cast the whole ternary.
- A `for` entry test is `sltu $at,$zero,n; beqz $at`; a `do`/`while` inside `if (n != 0)`
  gives `beqz n`.
- An index loop that recomputes `sll i,2` each pass, where yours keeps a byte offset in a saved
  register, comes from a unit built with strength reduction off:
  `#pragma opt_strength_reduction off`. It applies file-wide, so recheck every function there.

## Registers and types

- Declaration order of locals steers register allocation.
- `bool` locals give `daddiu`; `unsigned char` locals match byte loads.
- Float constants become locals declared at the top of the function (`float zero = 0.0f;`).
  One float local can fix the literal load order.
- Clamp as `if (v > 100U) v = 100;`. A clamp into a separate local can match where an
  in-place clamp does not.
- Argument reads that keep the original load order may need raw reads into locals first.

## Classes and data

- A subobject destructor called with `a1 = -1` is a data member, not a base class (flag 0).
- A file-local callee or data object (`STB_LOCAL`) is `static` in the source.
- Function-local `static` arrays match per-function data.
- Pass the symbol itself, never symbol-minus-offset.
- A callee's mangled name fixes its parameter types. When the C++ type adds an extension the
  original lacks (`unsigned short` into `unsigned int` gives an `andi`), call the mangled
  symbol through an `extern "C"` declaration.

## Tool settings

- Register scheduling depends on the source and object path lengths. Score a copy with the
  same path length as the real file, or a copy can match while the real build does not.
- A short loop with `slti; nop; bnez; nop` plus an extra `nop` after it matches only under
  older 3.0 builds. Note it in the draft and move on.
