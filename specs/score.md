# Score

A project-wide `rbit score` grades every unit that names a reference object. Long projects
keep thousands of matched functions, and a header edit or a neighbouring change can break one
silently, so the score remembers what matched and names what stopped matching.

## SCORE-1 Regressed functions are named

A project-wide score records each matched function in `reprobit.matched` and marks a recorded function that no longer matches as regressed.

Given a unit whose function `_kept` matched in an earlier project-wide score
When a later build changes `_kept` and `rbit score` runs again
Then `_kept` carries `regressed: true` and the summary counts 1 under `regressed`

## SCORE-2 The regression gate

`rbit score --regressions` fails only when a recorded function regressed.

Given a unit with one matched and one different function, scored once
When `rbit score --regressions` runs on the same build
Then it exits 0 though a function differs

Given the matched function then stops matching
When `rbit score --regressions` runs
Then it exits 1

## SCORE-3 Forgetting a regression

`rbit score --forget` drops each graded function that no longer matches from `reprobit.matched`.

Given a recorded function that regressed
When `rbit score --forget` runs and then `rbit score --regressions`
Then the second run exits 0 and counts 0 under `regressed`
