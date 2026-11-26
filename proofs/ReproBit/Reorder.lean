import ReproBit.Basic

/-!
Theorem 2: reordering instructions that do not conflict preserves behaviour.
A state assigns a value to each abstract location (registers, flags, memory
regions).
-/

namespace ReproBit

variable {I Loc Val : Type}

/-- Semantics respects footprints: an instruction changes only locations it
writes, and what it writes depends only on locations it reads. -/
structure Respects (exec : I → (Loc → Val) → (Loc → Val)) (reads writes : I → List Loc) :
    Prop where
  frame : ∀ i s l, l ∉ writes i → exec i s l = s l
  depends : ∀ i s t, (∀ l ∈ reads i, s l = t l) → ∀ l ∈ writes i, exec i s l = exec i t l

/-- `i` and `j` conflict: one writes what the other reads or writes. -/
def Conflict (reads writes : I → List Loc) (i j : I) : Prop :=
  (∃ l, l ∈ writes i ∧ (l ∈ reads j ∨ l ∈ writes j)) ∨ (∃ l, l ∈ writes j ∧ l ∈ reads i)

/-- Decidable form of `Conflict`. -/
def conflictb [DecidableEq Loc] (reads writes : I → List Loc) (i j : I) : Bool :=
  (writes i).any (fun l => (reads j).contains l || (writes j).contains l) ||
    (writes j).any (fun l => (reads i).contains l)

theorem not_conflict_of_conflictb [DecidableEq Loc] {reads writes : I → List Loc} {i j : I}
    (h : conflictb reads writes i j = false) : ¬ Conflict reads writes i j := by
  simp only [conflictb, Bool.or_eq_false_iff, List.any_eq_false, Bool.or_eq_true,
    List.contains_iff_mem, not_or] at h
  rintro (⟨l, hl, hr⟩ | ⟨l, hl, hr⟩)
  · rcases hr with hr | hr
    · exact (h.1 l hl).1 hr
    · exact (h.1 l hl).2 hr
  · exact h.2 l hl hr

variable {exec : I → (Loc → Val) → (Loc → Val)} {reads writes : I → List Loc}

/-- Theorem 2a: two adjacent non-conflicting instructions swap. -/
theorem swap_nonconflicting (h : Respects exec reads writes) {i j : I}
    (hc : ¬ Conflict reads writes i j) (s : Loc → Val) :
    exec j (exec i s) = exec i (exec j s) := by
  funext l
  have hww : ¬ (l ∈ writes i ∧ l ∈ writes j) := fun ⟨a, b⟩ => hc (Or.inl ⟨l, a, Or.inr b⟩)
  -- `a` writes `l`, `b` does not: `b` leaves `l` alone and `a`'s inputs.
  have key : ∀ {a b : I}, l ∈ writes a → l ∉ writes b → (∀ x, x ∈ writes b → x ∉ reads a) →
      exec b (exec a s) l = exec a (exec b s) l := by
    intro a b ha hb hd
    rw [h.frame b _ _ hb]
    exact h.depends a _ _ (fun x hx => (h.frame b s x (fun hw => hd x hw hx)).symm) l ha
  by_cases hi : l ∈ writes i
  · have hj : l ∉ writes j := fun hj => hww ⟨hi, hj⟩
    exact key hi hj (fun x hx hr => hc (Or.inr ⟨x, hx, hr⟩))
  · by_cases hj : l ∈ writes j
    · refine (key hj hi (fun x hx hr => hc (Or.inl ⟨x, hx, Or.inl hr⟩))).symm
    · rw [h.frame j _ _ hj, h.frame i _ _ hi, h.frame i _ _ hi, h.frame j _ _ hj]

/-- An instruction commuting with every instruction of a prefix moves to its front. -/
theorem run_move_front (h : Respects exec reads writes) {p : I} :
    ∀ (Q1 Q2 : List I), (∀ a ∈ Q1, ¬ Conflict reads writes a p) → ∀ s,
      run exec (Q1 ++ p :: Q2) s = run exec (Q1 ++ Q2) (exec p s) := by
  intro Q1
  induction Q1 with
  | nil => intro Q2 _ s; rfl
  | cons a Q1 ih =>
    intro Q2 hQ s
    simp only [List.cons_append, run_cons]
    rw [ih Q2 (fun b hb => hQ b (List.mem_cons_of_mem _ hb)),
      ← swap_nonconflicting h (hQ a List.mem_cons_self)]

/-- Core lemma on indices. `f` names the instruction at each index; `B` is the
original order (increasing indices); `π` is the new order. If `π` is a
permutation of `B` and each pair `p` before `q` in `π` is in original order or
non-conflicting, the programs agree. -/
theorem run_reorder (h : Respects exec reads writes) (f : Nat → I) :
    ∀ (B π : List Nat), B.Pairwise (· < ·) → π.Perm B →
      π.Pairwise (fun p q => p < q ∨ ¬ Conflict reads writes (f p) (f q)) →
      ∀ s, run exec (π.map f) s = run exec (B.map f) s := by
  intro B
  induction B with
  | nil =>
    intro π _ hp _ s
    rw [List.perm_nil.1 hp]
  | cons b B ih =>
    intro π hB hp hr s
    have hbπ : b ∈ π := hp.symm.subset List.mem_cons_self
    obtain ⟨Q1, Q2, rfl⟩ := List.append_of_mem hbπ
    have hB' := List.pairwise_cons.1 hB
    have hfront : ∀ a ∈ Q1, ¬ Conflict reads writes (f a) (f b) := by
      intro a ha
      have hab := (List.pairwise_append.1 hr).2.2 a ha b List.mem_cons_self
      have hmem : a ∈ b :: B := hp.subset (List.mem_append_left _ ha)
      rcases hab with hlt | hnc
      · rcases List.mem_cons.1 hmem with rfl | hmB
        · exact absurd hlt (Nat.lt_irrefl _)
        · exact absurd (Nat.lt_trans hlt (hB'.1 a hmB)) (Nat.lt_irrefl _)
      · exact hnc
    have hperm : (Q1 ++ Q2).Perm B := by
      have := List.perm_middle (a := b) (l₁ := Q1) (l₂ := Q2)
      exact List.perm_cons b |>.1 (this.symm.trans hp)
    have hsub : (Q1 ++ Q2).Pairwise (fun p q => p < q ∨ ¬ Conflict reads writes (f p) (f q)) :=
      hr.sublist (by simp)
    rw [List.map_append, List.map_cons,
      run_move_front h _ _ (by simpa using hfront)]
    rw [← List.map_append]
    simp only [List.map_cons, run_cons]
    exact ih _ hB'.2 hperm hsub _

/-- The checker's pair test: `p` before `q` is in original order or its
instructions do not conflict. -/
def okPairs [DecidableEq Loc] (reads writes : I → List Loc) (f : Nat → I) :
    List Nat → Bool
  | [] => true
  | p :: rest =>
    rest.all (fun q => decide (p < q) || !conflictb reads writes (f p) (f q)) &&
      okPairs reads writes f rest

theorem okPairs_sound [DecidableEq Loc] {f : Nat → I} :
    ∀ π : List Nat, okPairs reads writes f π = true →
      π.Pairwise (fun p q => p < q ∨ ¬ Conflict reads writes (f p) (f q)) := by
  intro π
  induction π with
  | nil => intro _; exact List.Pairwise.nil
  | cons p rest ih =>
    intro hok
    simp only [okPairs, Bool.and_eq_true, List.all_eq_true, Bool.or_eq_true, decide_eq_true_eq,
      Bool.not_eq_true'] at hok
    refine List.pairwise_cons.2 ⟨fun q hq => ?_, ih hok.2⟩
    rcases hok.1 q hq with h1 | h1
    · exact Or.inl h1
    · exact Or.inr (not_conflict_of_conflictb h1)

/-- Theorem 2b: if `π` permutes the indices of `0 … n-1` and every pair that
`π` inverts is non-conflicting, then running the instructions in `π` order
equals running them in index order. -/
theorem reorder_sound (h : Respects exec reads writes) (f : Nat → I) (n : Nat)
    (π : List Nat) (hp : π.Perm (List.range n))
    (hr : π.Pairwise (fun p q => p < q ∨ ¬ Conflict reads writes (f p) (f q))) (s : Loc → Val) :
    run exec (π.map f) s = run exec ((List.range n).map f) s :=
  run_reorder h f _ π List.pairwise_lt_range hp hr s

/-- The program `P` as a function on indices. -/
def at' [Inhabited I] (P : List I) (k : Nat) : I := P[k]!

theorem map_at'_range [Inhabited I] (P : List I) :
    (List.range P.length).map (at' P) = P := by
  apply List.ext_getElem
  · simp
  · intro k h1 h2
    simp [at', h2]

/-- Decidable checker: `π` is a permutation of `P`'s indices and every pair it
inverts has non-conflicting footprints. `Q = π.map (P[·]!)` is the reordered program. -/
def reorderOk [DecidableEq Loc] [Inhabited I] (reads writes : I → List Loc)
    (P : List I) (π : List Nat) : Bool :=
  π.isPerm (List.range P.length) && okPairs reads writes (at' P) π

/-- The checker is sound: if it accepts, the reordered program computes the same
final state as `P` from every initial state. -/
theorem reorderOk_sound [DecidableEq Loc] [Inhabited I] (h : Respects exec reads writes)
    (P : List I) (π : List Nat) (hok : reorderOk reads writes P π = true) (s : Loc → Val) :
    run exec (π.map (at' P)) s = run exec P s := by
  simp only [reorderOk, Bool.and_eq_true] at hok
  have := reorder_sound h (at' P) P.length π (List.isPerm_iff.1 hok.1) (okPairs_sound _ hok.2) s
  rwa [map_at'_range] at this

end ReproBit
