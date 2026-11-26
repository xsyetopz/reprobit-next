/-!
Theorem 3: renaming registers instruction by instruction preserves behaviour
when a register correspondence holds at every reachable point.

Both programs share their points (instruction indices) and control flow
graph. At each point, the old instruction reads `reads` and writes
`writes`; its renamed form reads and writes `rho` of them. `σ p` lists pairs
`(n, o)`: on reaching `p`, the new program's register `n` holds the value of
the old program's register `o`. The checker in
`crates/reprobit-msvc-4x/src/rename.rs` computes `σ` as a fixpoint over the
graph and checks `Checked`.
-/

namespace ReproBit

/-- One old instruction and its renamed form at the same point. -/
structure Renamed (R V : Type) where
  reads : List R
  writes : List R
  rho : R → R
  execOld : (R → V) → (R → V)
  execNew : (R → V) → (R → V)
  nextOld : (R → V) → Option Nat
  nextNew : (R → V) → Option Nat

variable {R V : Type}

/-- The renamed instruction computes what the old one did, on renamed
registers, and goes to the same successor; each leaves what it does not
write alone. -/
structure Renamed.Sound (i : Renamed R V) : Prop where
  agree : ∀ s s', (∀ r ∈ i.reads, s' (i.rho r) = s r) →
    (∀ w ∈ i.writes, i.execNew s' (i.rho w) = i.execOld s w) ∧ i.nextNew s' = i.nextOld s
  frameOld : ∀ s x, x ∉ i.writes → i.execOld s x = s x
  frameNew : ∀ s' x, (∀ w ∈ i.writes, x ≠ i.rho w) → i.execNew s' x = s' x

/-- Every pair in `σ` holds between old state `s` and new state `s'`. -/
def Holds (σ : List (R × R)) (s s' : R → V) : Prop := ∀ p ∈ σ, s' p.1 = s p.2

/-- A pair that holds after `i`: kept from `σ` because `i` overwrites
neither side, or written by `i`. -/
def Post (i : Renamed R V) (σ : List (R × R)) (n o : R) : Prop :=
  ((n, o) ∈ σ ∧ ∀ w ∈ i.writes, n ≠ i.rho w ∧ o ≠ w) ∨ (∃ w ∈ i.writes, n = i.rho w ∧ o = w)

theorem post_holds {i : Renamed R V} (h : i.Sound) {σ : List (R × R)} {s s' : R → V}
    (hs : Holds σ s s') (hr : ∀ r ∈ i.reads, (i.rho r, r) ∈ σ) {n o : R}
    (hp : Post i σ n o) : i.execNew s' n = i.execOld s o := by
  rcases hp with ⟨hm, hw⟩ | ⟨w, hw, hn, ho⟩
  · rw [h.frameNew s' n (fun w hw' => (hw w hw').1),
      h.frameOld s o (fun hw' => (hw o hw').2 rfl)]
    exact hs _ hm
  · rw [hn, ho]
    exact (h.agree s s' (fun r hm => hs _ (hr r hm))).1 w hw

/-- Runs from point `p` for at most `f` instructions; `some` final state
once an instruction has no successor. -/
def runOld (prog : Nat → Renamed R V) : Nat → Nat → (R → V) → Option (R → V)
  | 0, _, _ => none
  | f + 1, p, s =>
    match (prog p).nextOld s with
    | none => some ((prog p).execOld s)
    | some q => runOld prog f q ((prog p).execOld s)

/-- `runOld` for the renamed program. -/
def runNew (prog : Nat → Renamed R V) : Nat → Nat → (R → V) → Option (R → V)
  | 0, _, _ => none
  | f + 1, p, s =>
    match (prog p).nextNew s with
    | none => some ((prog p).execNew s)
    | some q => runNew prog f q ((prog p).execNew s)

/-- What the checker establishes over the points `reach` it found
reachable, which include every successor of a reachable point. -/
structure Checked (prog : Nat → Renamed R V) (σ : Nat → List (R × R)) (obs : List R)
    (reach : Nat → Prop) : Prop where
  sound : ∀ p, (prog p).Sound
  closed : ∀ p s q, reach p → (prog p).nextOld s = some q → reach q
  reads : ∀ p, reach p → ∀ r ∈ (prog p).reads, ((prog p).rho r, r) ∈ σ p
  edge : ∀ p, reach p → ∀ s q, (prog p).nextOld s = some q →
    ∀ pr ∈ σ q, Post (prog p) (σ p) pr.1 pr.2
  exit : ∀ p, reach p → ∀ s, (prog p).nextOld s = none → ∀ r ∈ obs, Post (prog p) (σ p) r r

/-- Theorem 3: from corresponding states at a reachable point, the renamed
program finishes whenever the old one does, with equal observed registers. -/
theorem run_agrees {prog : Nat → Renamed R V} {σ : Nat → List (R × R)} {obs : List R}
    {reach : Nat → Prop} (hc : Checked prog σ obs reach) :
    ∀ f p (s s' t : R → V), reach p → Holds (σ p) s s' → runOld prog f p s = some t →
      ∃ t', runNew prog f p s' = some t' ∧ ∀ r ∈ obs, t' r = t r := by
  intro f
  induction f with
  | zero => intro p s s' t _ _ h; simp [runOld] at h
  | succ f ih =>
    intro p s s' t hp hs h
    have hnext := ((hc.sound p).agree s s' (fun r hm => hs _ (hc.reads p hp r hm))).2
    simp only [runOld] at h
    simp only [runNew, hnext]
    cases hq : (prog p).nextOld s with
    | none =>
      rw [hq] at h
      simp only [Option.some.injEq] at h
      refine ⟨_, rfl, fun r hr => ?_⟩
      rw [← h]
      exact post_holds (hc.sound p) hs (hc.reads p hp) (hc.exit p hp s hq r hr)
    | some q =>
      rw [hq] at h
      exact ih q _ _ t (hc.closed p s q hp hq)
        (fun pr hm => post_holds (hc.sound p) hs (hc.reads p hp) (hc.edge p hp s q hq pr hm)) h

/-- Theorem 3 from the entry: with only identity pairs at the entry, both
programs run from the same state end with equal observed registers. -/
theorem rename_sound {prog : Nat → Renamed R V} {σ : Nat → List (R × R)} {obs : List R}
    {reach : Nat → Prop} (hc : Checked prog σ obs reach) {e : Nat} (he : reach e)
    (hid : ∀ pr ∈ σ e, pr.1 = pr.2) (f : Nat) (s t : R → V) (h : runOld prog f e s = some t) :
    ∃ t', runNew prog f e s = some t' ∧ ∀ r ∈ obs, t' r = t r :=
  run_agrees hc f e s s t he (fun pr hm => by rw [hid pr hm]) h

end ReproBit
