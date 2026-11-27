---
trigger: always_on
description: ReproBit hard rules for matching decompilation projects.
---

# ReproBit hard rules

- Never commit `reference/`, `.reprobit/`, compilers, or disc images (`.iso`, `.cue`, `.bin`, `.gcm`, `.rvz`, `.wbfs`). They hold copyrighted or machine-local data. Stage by explicit path.
- Only `rbit verify` reporting `identical` certifies a match. A clean objdiff score, a passing build or a matching hash from another tool does not.
- Write C or C++ for every function C can express. Inline assembly is not a match; it hides the difference.
- Never copy bytes or instruction words from `reference/` into a source file.
- Exit code 1 from `rbit diff` or `rbit verify` is a negative result, not a tool failure.
- Run `rbit` through `rbit --json` when a script or agent reads the output.
- Header and shared-type changes belong to the lead. A worker owns one source file.
- Before proposing a change to a community project, read its contribution rules; some refuse AI-written code. Label AI-assisted commits with an `Assisted-by:` trailer.
- Load the `reprobit` skill for setup and builds, `reprobit-match` for one function, `reprobit-coordinate` for a queue of jobs.
