import ReproBit.Basic

/-!
Theorem 1: renaming registers by a permutation preserves behaviour.
A state is a register file plus memory; a permutation acts on the register
file only.
-/

namespace ReproBit

/-- A bijection on registers, with its inverse. -/
structure Perm (R : Type) where
  fwd : R → R
  bwd : R → R
  bwd_fwd : ∀ r, bwd (fwd r) = r
  fwd_bwd : ∀ r, fwd (bwd r) = r

/-- The inverse permutation. -/
def Perm.symm {R : Type} (π : Perm R) : Perm R := ⟨π.bwd, π.fwd, π.fwd_bwd, π.bwd_fwd⟩

/-- Registers hold values of type `V`; `M` is everything else. -/
structure State (R V M : Type) where
  regs : R → V
  mem : M

variable {R V M I O : Type}

/-- `π • s`: the value `s` keeps in `r` now lives in `π r`. Memory is untouched. -/
def Perm.act (π : Perm R) (s : State R V M) : State R V M :=
  ⟨fun r => s.regs (π.bwd r), s.mem⟩

/-- Instruction semantics commute with renaming: the hypothesis the Rust checker
establishes by refusing implicit register uses. -/
def Equivariant (exec : I → State R V M → State R V M) (rename : Perm R → I → I) : Prop :=
  ∀ π i s, exec (rename π i) (π.act s) = π.act (exec i s)

/-- Theorem 1a: a renamed program run on a renamed state ends in the renamed state. -/
theorem run_equivariant {exec : I → State R V M → State R V M} {rename : Perm R → I → I}
    (h : Equivariant exec rename) (π : Perm R) (P : List I) (s : State R V M) :
    run exec (P.map (rename π)) (π.act s) = π.act (run exec P s) := by
  induction P generalizing s with
  | nil => rfl
  | cons i P ih => simp only [List.map_cons, run_cons]; rw [h π i s]; exact ih _

/-- Two final states are indistinguishable to a caller: registers outside `S`
and the observation of memory agree. -/
def Agree (S : R → Prop) (obs : M → O) (a b : State R V M) : Prop :=
  (∀ r, ¬ S r → a.regs r = b.regs r) ∧ obs a.mem = obs b.mem

/-- Theorem 1b (callee-saved corollary). `π` moves only registers in `S`;
`P` (a) preserves every register in `S`, and (b) has an `Agree`-result that does
not depend on the initial registers in `S`. Then the renamed program has the
same result from any initial state and also preserves every register in `S`. -/
theorem callee_saved {exec : I → State R V M → State R V M} {rename : Perm R → I → I}
    (h : Equivariant exec rename) (S : R → Prop) (obs : M → O) (π : Perm R)
    (hπ : ∀ r, ¬ S r → π.fwd r = r) (P : List I)
    (hpres : ∀ s r, S r → (run exec P s).regs r = s.regs r)
    (hind : ∀ s t, (∀ r, ¬ S r → s.regs r = t.regs r) → s.mem = t.mem →
      Agree S obs (run exec P s) (run exec P t))
    (s : State R V M) :
    Agree S obs (run exec (P.map (rename π)) s) (run exec P s) ∧
      ∀ r, S r → (run exec (P.map (rename π)) s).regs r = s.regs r := by
  have hb : ∀ r, ¬ S r → π.bwd r = r := fun r hr => by
    have := π.bwd_fwd r
    rw [hπ r hr] at this
    exact this
  -- `t` is `s` with the renaming undone, so `π • t = s`.
  let t : State R V M := π.symm.act s
  have hs : π.act t = s := by
    cases s with
    | mk regs mem =>
      simp only [t, Perm.act, Perm.symm]
      congr 1
      funext r
      exact congrArg regs (π.fwd_bwd r)
  have hrun : run exec (P.map (rename π)) s = π.act (run exec P t) := by
    rw [← run_equivariant h π P t, hs]
  have ht : ∀ r, ¬ S r → s.regs r = t.regs r := fun r hr => by
    simp only [t, Perm.act, Perm.symm, hπ r hr]
  refine ⟨?_, ?_⟩
  · obtain ⟨h1, h2⟩ := hind s t ht rfl
    rw [hrun]
    refine ⟨fun r hr => ?_, h2.symm⟩
    simp only [Perm.act, hb r hr]
    exact (h1 r hr).symm
  · intro r hr
    rw [hrun]
    simp only [Perm.act]
    have hS : S (π.bwd r) := by
      refine Classical.byContradiction fun hn => ?_
      have := hπ _ hn
      rw [π.fwd_bwd] at this
      exact hn (this ▸ hr)
    rw [hpres t _ hS]
    simp only [t, Perm.act, Perm.symm, π.fwd_bwd]
