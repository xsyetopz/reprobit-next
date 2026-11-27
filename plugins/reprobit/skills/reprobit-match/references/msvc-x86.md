# Microsoft Visual C++ 4.x and 5.0 (i386)

What ReproBit already handles for these compilers, and when to reach for it.

- Register allocation that differs only in `ebx`, `esi`, and `edi`: `rbit grind` tries
  renaming them, because every calling convention preserves all three.
- A difference that appears only after other declarations changed: the compiler's internal
  numbering moved. `rbit grind` searches declaration-only preludes, smallest first.
- MSVC 4.x instruction order, register choice, and operand order that no source edit reaches:
  add `msvc-4x` to `toolchain.tools`. Grind then asks it for `schedule`, `rename`,
  `exchange`, and `region` rewrites, and saves one only when the function matches.
- x87 addition with three or more terms: keep the original's term order. Floating-point
  addition is not associative, so reordering terms can change the result.
- Fixed DOS paths and PE timestamps matter for byte identity; declare `normalize =
  ["pe-timestamps"]` on the target instead of editing the artifact.
- Function-level comparison of a whole executable: `reccmp` from the catalog.
