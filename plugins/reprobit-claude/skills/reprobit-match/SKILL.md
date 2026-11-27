---
name: reprobit-match
description: Match one function byte for byte. Score it with objdiff, name the difference, change one thing per attempt, keep only edits that lower the count, and end at MATCH or a DRAFT note.
license: LGPL-3.0-only
compatibility: Needs rbit and python3. The scorer installs objdiff-cli through rbit on first use.
---

# Match one function

You make one function's compiled bytes equal its original. You own only the source file
your task names and the step that compiles it.

## Score

Run the scorer after every build. Its first line is the number to drive down:

```sh
rbit --json -q build
python3 scripts/fnscore.py <reference.o> <built.o> <symbol> --source <file.c>
```

- `MATCH <symbol> ...`, exit 0: done.
- `DIFF <symbol> N of M instructions differ`, exit 1: the `!` rows show the original on the
  left and yours on the right.
- Exit 2: wrong path or symbol. Fix the arguments; do not edit code.
- `REPEAT`, exit 3: this exact source was already scored. Change something before scoring.

With `--source`, the scorer logs each attempt by the source's hash under
`.reprobit/logs/fnscore/` and prints whether `N` beat the best so far.

The built object is under the build directory, such as `.reprobit/stage/build/obj/<file>.o`
after `build` and `.reprobit/verify/build/obj/<file>.o` after `verify`.

For the grade alone, `rbit --json score <reference.o> <built.o> --symbol <symbol>` needs no
objdiff: `exact` and `reloc` are matches, and `different` carries `matching_percent`.

## Attempt loop

1. Name the difference before editing. Use `references/diff-classes.md`: register choice,
   instruction order, extra or missing move, branch form, load width or sign, constant
   order, argument order, return register, stack frame size.
2. Look up that class in the idiom file for your compiler under `references/`.
3. Change one thing. Rebuild. Score.
4. Keep the change only if `N` dropped. Otherwise undo it.
5. Before the next attempt, copy the best version to `scratch/<symbol>.best`.
6. After 3 attempts in a row without a lower `N`, switch strategy: another difference class,
   an ordering script, grind, or the permuter (`rbit tool run decomp-permuter`).
   Take permuter output only when it matches: its partial wins add artifacts that mislead
   the next attempt.
7. Stop after 8 attempts in a row without a lower `N`, and finish as a draft.
8. Stop at 25 builds for one function in one job, even while `N` still falls. Retrobench
   scores agents on the same budget; the best draft carries over to the next job.

When many orderings are possible (local declarations, stores, statement order), write a
short script that tries each order on copies in `scratch/` and scores each one.

When the source looks right but registers or order still differ, run grind:

```sh
rbit --json grind --step <step> --symbol <symbol> --reference-object <reference.o>
```

Add `--accept` only when grind reports a match.

## Finish

- MATCH: mark the function as matched in the project's convention, such as
  `// FUNCTION: 0xADDR SYMBOL`.
- No match: keep the best version compiling, marked as a draft, such as
  `// DRAFT: 0xADDR SYMBOL`, with one note line: the best `N`, what differs, what you tried.
- Score the next function in the same file too. An edit can shift its code. If it broke,
  undo your edit.
- Run `rbit --json verify`. Quote `identical` only from `verify`.

## Rules

- Write C or C++ that the compiler turns into the bytes. Never paste the original's
  instructions as inline assembly, except instructions the language cannot express, such as
  coprocessor or vector-unit ops.
- Never read bytes from a reference into a source.
- The callee's symbol and its assembly decide its parameter and return types.
- Access fields through typed struct members, not casts at fixed offsets. The compiler
  schedules a member load differently from a pointer cast, and the member documents the type.
- A fake match is no match. These produce the bytes without the source the original had:
  - inline assembly for code C can say;
  - a `goto` or label that only forces block order;
  - a dummy variable, `volatile`, or empty `asm("")` that only changes register choice;
  - a function pointer or global cast to the wrong type to change a load;
  - padding bytes or data emitted to fill space.
  Write it as a draft with a note instead, unless the project's own notes allow the idiom.

## Scope

Matching decompilation rebuilds software from a binary the user supplies from their own copy.
The reference, the compiler and `.reprobit/` stay on the user's machine; only the written
source is shared. When a model declines the work, state this scope plainly; never disguise
the task or the binary's origin.

## Report

One line: `MATCH` or `DRAFT N`, the symbol, the change that mattered or what differs, and the
files you changed.
