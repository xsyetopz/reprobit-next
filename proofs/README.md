# ReproBit proofs

Lean 4 soundness theorems for ReproBit rewrites. The package has no
dependencies (no Mathlib) and builds with `lake build` under the toolchain in
`lean-toolchain`. Every theorem uses only Lean core axioms (`propext`,
`Classical.choice`, `Quot.sound`); there is no `sorry`.

`rbit tool run lean:lake -- build`, run here, installs that Lean from the
catalog and checks every proof; the `Proofs` workflow runs it on each change.

| Module | Contents |
| --- | --- |
| `ReproBit/Basic.lean` | `run`: execute a program left to right |
| `ReproBit/Permutation.lean` | `run_equivariant`, `callee_saved` |
| `ReproBit/Reorder.lean` | `swap_nonconflicting`, `reorder_sound`, `reorderOk_sound` |
| `ReproBit/Correspondence.lean` | `post_holds`, `run_agrees`, `rename_sound` |

## Theorem 1: register permutation (`crates/reprobit/src/rewrite.rs`)

A state is a register file plus memory. A permutation `π` renames registers in
instructions and moves register contents in states; memory is untouched.

- `run_equivariant`: if each instruction's semantics is equivariant
  (`exec (rename π i) (π • s) = π • exec i s`), a renamed program run on a
  renamed state ends in the renamed final state.
- `callee_saved`: let `S` be a register set and `π` fix every register outside
  `S`. If the original program `P` (a) leaves every register in `S` at its
  initial value and (b) produces a result (registers outside `S`, plus an
  observation `obs` of memory that excludes the dead stack region) that does
  not depend on the initial registers in `S`, then the renamed program
  produces the same result from every initial state and also preserves every
  register in `S`.

Hypotheses and the Rust check behind each:

| Hypothesis | Established by |
| --- | --- |
| Equivariance | `rename` refuses any instruction that uses a renamed register implicitly (the `used_registers` versus operand-named check, e.g. `rep movsd`), any byte register the target lacks, and any indirect jump through a table. Every renamed register is therefore an explicit operand, so renaming operands is the whole effect. |
| `π` is a permutation | `rename` checks that the key set of the map equals its value set. |
| `π` fixes everything outside `S` | `rename` checks both ends of every pair against `allowed`, the set `renamable` returns (`esi`, `edi`, plus `ebx` outside ELF). |
| (a) `S` preserved, (b) result independent of `S` inputs | Not checked by the Rust code. They are the calling-convention assumption in the module comment of `rewrite.rs`: callers expect `ebx`, `esi`, `edi` back unchanged, and no caller or callee passes arguments or results in them. The user holds these. |

## Theorem 2: instruction reordering

A state maps abstract locations (registers, flags, memory regions) to values.
Each instruction has a read list and a write list.

- Hypothesis `Respects`: an instruction leaves locations outside its writes
  unchanged, and the values it writes depend only on its reads.
- `Conflict i j`: `i` writes something `j` reads or writes, or `j` writes
  something `i` reads.
- `swap_nonconflicting`: two adjacent non-conflicting instructions swap
  without changing the final state.
- `reorder_sound`: instructions are named by index. If `π` is a permutation
  of `0 … n-1` and every pair that `π` places out of original order is
  non-conflicting, then running in `π` order equals running in index order.
  Indices make duplicate instructions distinct occurrences.
- `reorderOk` is a `Bool` checker on concrete lists: `π.isPerm (range n)` and,
  for every pair `p` before `q` in `π`, `p < q` or `conflictb` is false.
  `reorderOk_sound`: if it returns `true`, the reordered program
  `π.map (P[·]!)` computes the same final state as `P` from every initial
  state. `schedule::check` in `crates/reprobit-msvc-4x/src/schedule.rs` is this
  function over the add-on's footprints.

The `Respects` hypothesis is a modelling obligation on each instruction set's
footprint table: the Rust footprint function must list every location an
instruction can write, and every location its written values depend on.
Conservative footprints (extra reads or writes) stay sound; missing ones do
not. `schedule::decode` builds them from `iced-x86`'s tables: it adds a read for
each partial or conditional write and each undefined flag, and gives every
branch, branch target, and place another record names a write to a `Control`
location that every instruction reads, so those keep their place. Control flow
stays outside the straight-line model: because barriers never move, every
straight-line run between them is reordered on its own, which is the case the
theorem covers.

Memory is split into one location per stack byte, numbered from `esp` at the
run's start, and one for the rest. An access through a register other than
`esp` lists the rest and every stack byte the run names, since it may reach
any of them. The theorem's instructions are then the operations, not their
encodings: `[esp+d]` at a point where `esp` sits `depth` below the run's start
is the operation on byte `depth + d`, which reads no `esp`. `schedule::emit`
encodes each operation at its new place with `d' = d + depth - depth'`, and
`depth` there is the sum of the earlier `esp` adjustments. Those adjustments
(push, pop, `add` and `sub esp` by a constant) all read and write `esp`, so
they keep their order and each sees the same `depth`. That this encoding
computes the operation is an obligation on the Rust code, like the footprints,
and is not proved in Lean. Faults are outside the model: a moved load that
faults does so at its new place.

## Theorem 3: per-instruction register renames

Both programs share their instruction indices and control flow graph. Each
point holds an old instruction with `reads` and `writes` and its renamed form
under `rho`. `σ p` lists pairs `(n, o)`: on reaching `p`, the new program's
register `n` holds the old program's register `o`.

- `Renamed.Sound`: given inputs equal under `rho`, the renamed instruction
  writes `rho w` with the value the old one writes to `w`, takes the same
  successor, and leaves everything else alone.
- `Post i σ`: the pairs that hold after `i`, namely those `i` overwrites on
  neither side, plus `(rho w, w)` for each write.
- `Checked`: every point is sound, each read `r` finds `(rho r, r)` in `σ`,
  `σ` at a successor is within `Post` of its predecessor, and at an exit every
  observed register `r` has `(r, r)` in `Post`.
- `run_agrees` and `rename_sound`: if `Checked` holds and the entry has only
  identity pairs, the renamed program finishes whenever the old one does, with
  equal observed registers.

`rename::check` in `crates/reprobit-msvc-4x/src/rename.rs` computes `σ` as
the greatest fixpoint, met by intersection at joins, and checks `reads` and
`exit`. It observes the registers a caller sees at a `ret`, and all general
registers at other exits; a superset of the `ret` set, so `Checked` holds with
`obs` as the `ret` set. `Renamed.Sound` is an obligation on the Rust code, as
`Respects` is for Theorem 2: `rename::encode` requires every renamed register
to be an explicit operand, `rho` to be injective on the registers the
instruction uses, and the re-decoded instruction's footprint to equal `rho` of
the original's. Flags and memory are never renamed, so they are equal on both
sides. Calls are modelled by the calling convention, as in Theorem 1.
