# Locate

Retail binaries link vendor library code (console SDKs, C runtimes) from archives the
project can obtain, and that code is verified against its original objects rather than
decompiled. `rbit locate` finds where each library function sits in a binary, so progress can
count library code apart from decompiled code.

## LOCATE-1 A function placed once is found

A library function whose bytes appear at exactly one place in the binary's code, with relocated fields ignored, is reported `found` at that address.

Given an archive member defining `_load`, whose relocated operand the linker filled in
When `rbit locate` runs on a binary holding that code at address 4
Then `_load` is `found` at address 4

## LOCATE-2 Linked members

A member counts as `linked` only when every function it defines is `found`.

Given members `a.obj` with both functions in the binary and `b.obj` whose function is not
When `rbit locate --all` runs
Then `a.obj` is linked, `b.obj` is not, its function is `absent`, and the summary counts 1 linked

## LOCATE-3 Ambiguous places

A function whose bytes appear at more than one place is reported `ambiguous` with its number of places, never `found`.

Given a member defining `_zero`, whose bytes the binary holds twice
When `rbit locate` runs
Then `_zero` is `ambiguous` with 2 places and its member is not linked

## LOCATE-4 Shared places

A place where functions of several members are found stays `found` only for the one claimant whose member has every other function found, and is `ambiguous` for the rest.

Given `p.obj` with all its functions in the binary and `q.obj` whose other function is absent, both holding the same bytes once
When `rbit locate` runs
Then the `p.obj` function is `found`, the `q.obj` one is `ambiguous` and lists `p.obj` in `shared`

Given two one-function members `x.obj` and `y.obj` with the same bytes, placed once
When `rbit locate` runs
Then both functions are `ambiguous` and neither member is linked

## LOCATE-5 Unplaced members are counted, not listed

Without `--all`, a member with no function `found` or `ambiguous` is left out of `members` and counted under `unplaced`.

Given members `a.obj` with both functions in the binary and `b.obj` whose function is not
When `rbit locate` runs without `--all`
Then `members` holds `a.obj` and not `b.obj`, and `unplaced` is 1
