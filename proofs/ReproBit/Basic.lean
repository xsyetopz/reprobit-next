namespace ReproBit

/-- Runs a program left to right: each instruction maps one state to the next. -/
def run {I σ : Type} (exec : I → σ → σ) : List I → σ → σ
  | [], s => s
  | i :: p, s => run exec p (exec i s)

@[simp] theorem run_nil {I σ : Type} (exec : I → σ → σ) (s : σ) : run exec [] s = s := rfl

@[simp] theorem run_cons {I σ : Type} (exec : I → σ → σ) (i : I) (p : List I) (s : σ) :
    run exec (i :: p) s = run exec p (exec i s) := rfl

theorem run_append {I σ : Type} (exec : I → σ → σ) (p q : List I) (s : σ) :
    run exec (p ++ q) s = run exec q (run exec p s) := by
  induction p generalizing s with
  | nil => rfl
  | cons i p ih => exact ih _

end ReproBit
