/-!
The region simulator's normal forms keep a term's value. The simulator in
`crates/reprobit-msvc-4x/src/region/simulate.rs` compares the terms two
runs leave in each location; a location whose terms are equal holds equal
values whatever the start state, since a term's value is a function of the
term. Two runs reach equal terms through different orders only by the
rewrites below: a sum's terms are flattened and sorted, and the two
operands of a commuting operation are sorted.
-/

namespace ReproBit.Simulation

/-- A value over the start state's locations `L`: a start value, a
constant, a wrapping sum of terms plus a constant, or output `tag` of the
operation `code` on argument terms. -/
inductive Term (L : Type) where
  | start : L → Term L
  | const : BitVec 32 → Term L
  | sum : List (Term L) → BitVec 32 → Term L
  | op : Nat → Nat → List (Term L) → Term L

variable {L : Type}

mutual
/-- A term's value in start state `s`, with operations interpreted by `f`. -/
def eval (f : Nat → Nat → List (BitVec 32) → BitVec 32) (s : L → BitVec 32) :
    Term L → BitVec 32
  | .start l => s l
  | .const c => c
  | .sum ts c => (evalList f s ts).foldr (· + ·) c
  | .op code tag ts => f code tag (evalList f s ts)

/-- The values of a list of terms. -/
def evalList (f : Nat → Nat → List (BitVec 32) → BitVec 32) (s : L → BitVec 32) :
    List (Term L) → List (BitVec 32)
  | [] => []
  | t :: ts => eval f s t :: evalList f s ts
end

theorem evalList_eq_map (f : Nat → Nat → List (BitVec 32) → BitVec 32) (s : L → BitVec 32)
    (ts : List (Term L)) : evalList f s ts = ts.map (eval f s) := by
  induction ts with
  | nil => simp [evalList]
  | cons t ts ih => simp [evalList, ih]

/-- A wrapping sum does not depend on the order of its addends. -/
theorem foldr_add_perm {xs ys : List (BitVec 32)} (h : xs.Perm ys) (c : BitVec 32) :
    xs.foldr (· + ·) c = ys.foldr (· + ·) c := by
  induction h with
  | nil => rfl
  | cons x _ ih => simp [List.foldr, ih]
  | swap x y l =>
    simp only [List.foldr]
    rw [← BitVec.add_assoc, BitVec.add_comm y x, BitVec.add_assoc]
  | trans _ _ ih₁ ih₂ => exact ih₁.trans ih₂

/-- Sorting a sum's terms keeps its value. -/
theorem sum_perm (f : Nat → Nat → List (BitVec 32) → BitVec 32) (s : L → BitVec 32)
    {ts us : List (Term L)} (h : ts.Perm us) (c : BitVec 32) :
    eval f s (.sum ts c) = eval f s (.sum us c) := by
  simp only [eval, evalList_eq_map]
  exact foldr_add_perm (h.map _) c

/-- A wrapping sum from `c` is the sum from zero plus `c`. -/
theorem foldr_add_eq (xs : List (BitVec 32)) (c : BitVec 32) :
    xs.foldr (· + ·) c = xs.foldr (· + ·) 0 + c := by
  induction xs with
  | nil => simp
  | cons x xs ih => simp only [List.foldr, ih, BitVec.add_assoc]

/-- Flattening a sum inside a sum keeps its value. -/
theorem sum_flatten (f : Nat → Nat → List (BitVec 32) → BitVec 32) (s : L → BitVec 32)
    (ts us : List (Term L)) (c d : BitVec 32) :
    eval f s (.sum (.sum ts c :: us) d) = eval f s (.sum (ts ++ us) (c + d)) := by
  simp only [eval, evalList_eq_map, List.map_cons, List.map_append, List.foldr,
    List.foldr_append]
  rw [foldr_add_eq (List.map (eval f s) us) (_ + d), foldr_add_eq (List.map (eval f s) ts) c,
    foldr_add_eq (List.map (eval f s) ts) (_ + (c + d))]
  rw [foldr_add_eq (List.map (eval f s) us) d]
  ac_rfl

/-- Sorting the two operands of an operation that commutes keeps its value. -/
theorem op_swap (f : Nat → Nat → List (BitVec 32) → BitVec 32) (s : L → BitVec 32)
    (code tag : Nat) (commutes : ∀ a b, f code tag [a, b] = f code tag [b, a])
    (a b : Term L) : eval f s (.op code tag [a, b]) = eval f s (.op code tag [b, a]) := by
  simp only [eval, evalList]
  exact commutes _ _

end ReproBit.Simulation
