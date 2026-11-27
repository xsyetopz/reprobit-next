# Difference classes

Name the class first, then search the idiom file for it. The left column of the scorer is
the original.

| Class | What the rows show | First things to try |
| --- | --- | --- |
| Register choice | Same instructions, other registers | Order of local declarations; a separate local for a reused value; `grind` with `--family registers` |
| Instruction order | Same instructions, other order | Statement order; split or join expressions; ternary instead of if/else |
| Extra move | Yours copies a value the original uses in place | Remove a temporary; use the parameter directly |
| Missing move | The original copies into a new register | Add a local for the value |
| Branch form | `bnel` against `bne`, `jz` against `jnz`, inverted test | Invert the condition; early return instead of a wrapping `if`; `goto` into a loop's test |
| Loop shape | Test at the top against a jump to a test at the bottom | `for` against `while` against `do`/`while`; a goto-entered loop |
| Load width or sign | `lb` against `lbu`, `movsx` against `movzx` | Type of the variable or field: `char`, `unsigned char`, `short`, `bool` |
| Constant order | Same constants loaded in another order | Hoist constants into locals in the original's order |
| Argument order | Arguments evaluated in another order | Read arguments into locals first; swap the order of side effects |
| Return register | Value returned in another register, or a needless move | Return type of this function or its callee |
| Frame size | Stack adjust differs | Number and size of locals; a local array; an inlined callee |
| Data reference | Address of `symbol+offset` against `symbol` | Reference the symbol itself; make file-local data `static` |
| Whole function shifted | Every row differs by an offset | A caller or neighbour changed; score the earlier function first |
