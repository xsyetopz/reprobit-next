---
description: Match one function byte for byte with the reprobit-match skill
argument-hint: "<symbol> <source file>"
---

# Match one function

Match `$1` in `$2` with the reprobit-match skill. Score with its fnscore.py and `--source $2`, change one thing per attempt, keep only edits that lower the count, stop at the skill's budgets, and end with `rbit --json verify`. Edit only `$2`.
