#!/usr/bin/env python3
"""Score one function against its reference with objdiff.

Prints `MATCH <symbol> ...` and exits 0, or `DIFF <symbol> N of M instructions differ`
followed by the differing rows and exits 1. Exit 2 is an error, such as a missing symbol.
The count is the number to drive down: keep an edit only when it falls.

With `--source FILE`, each attempt is logged by the source's hash under
`.reprobit/logs/fnscore/`. A source already scored exits 3 with `REPEAT`, because
resubmitting the same code cannot change the result.

Usage: fnscore.py REFERENCE_OBJECT BUILT_OBJECT SYMBOL [--source FILE] [--rows N]
"""

import hashlib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import NoReturn

LOG_DIR = Path(".reprobit/logs/fnscore")


def fail(message: str) -> NoReturn:
    """Exits 2, the error code, so an error never reads as a DIFF."""
    print(message, file=sys.stderr)
    raise SystemExit(2)


def objdiff(reference: str, built: str, symbol: str) -> dict:
    """Runs objdiff-cli one-shot and returns its JSON; rbit installs it on first use."""
    args = ["diff", "-1", reference, "-2", built, "-o", "-", symbol]
    direct = shutil.which("objdiff-cli")
    command = [direct, *args] if direct else ["rbit", "-q", "tool", "run", "objdiff-cli", "--", *args]
    done = subprocess.run(command, capture_output=True, text=True, stdin=subprocess.DEVNULL)
    if done.returncode != 0:
        fail(f"error: objdiff-cli failed: {done.stderr.strip()[:500]}")
    return json.loads(done.stdout)


def find(side: dict, symbol: str) -> dict | None:
    return next((s for s in side.get("symbols", []) if s.get("name") == symbol), None)


def text(row: dict) -> str:
    return (row.get("instruction") or {}).get("formatted", "")


def option(argv: list[str], name: str) -> str | None:
    if name not in argv:
        return None
    at = argv.index(name)
    if at + 1 >= len(argv):
        fail(f"error: {name} needs a value")
    value = argv[at + 1]
    del argv[at : at + 2]
    return value


def history(symbol: str) -> tuple[Path, list[list[str]]]:
    path = LOG_DIR / (re.sub(r"[^A-Za-z0-9_.-]", "_", symbol) + ".tsv")
    rows = [line.split("\t") for line in path.read_text().splitlines()] if path.is_file() else []
    return path, rows


def main(argv: list[str]) -> int:
    try:
        rows = int(option(argv, "--rows") or 30)
    except ValueError:
        fail("error: --rows needs a number")
    source = option(argv, "--source")
    if len(argv) != 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    reference, built, symbol = argv
    log, attempts = None, []
    digest = ""
    if source:
        if not Path(source).is_file():
            fail(f"error: {source} is not a file")
        digest = hashlib.sha256(Path(source).read_bytes()).hexdigest()[:16]
        log, attempts = history(symbol)
        for number, (seen, score) in enumerate(attempts, 1):
            if seen == digest:
                print(f"REPEAT {symbol}: this source is attempt {number} ({score}); change something")
                return 3
    result = objdiff(reference, built, symbol)
    left, right = find(result.get("left", {}), symbol), find(result.get("right", {}), symbol)
    if left is None or right is None:
        missing = "reference" if left is None else "built object"
        print(f"error: {symbol} is not in the {missing}", file=sys.stderr)
        return 2
    ours, theirs = right.get("instructions", []), left.get("instructions", [])
    total = max(len(ours), len(theirs))
    differing = [i for i in range(total) if any(
        (side[i] if i < len(side) else {}).get("diff_kind") for side in (theirs, ours)
    )]
    percent = left.get("match_percent", 0.0)
    matched = not differing and left.get("size") == right.get("size")
    if log is not None:
        log.parent.mkdir(parents=True, exist_ok=True)
        with log.open("a") as out:
            out.write(f"{digest}\t{'MATCH' if matched else f'DIFF {len(differing)}'}\n")
        scores = [int(s.split()[1]) for _, s in attempts if s.startswith("DIFF ")]
        if scores and not matched:
            best = min(scores)
            trend = "lower" if len(differing) < best else "not lower"
            print(f"attempt {len(attempts) + 1}: {len(differing)} is {trend} than the best {best}")
    if matched:
        print(f"MATCH {symbol} {percent:.2f}% ({total} instructions)")
        return 0
    print(f"DIFF {symbol} {len(differing)} of {total} instructions differ, {percent:.2f}%")
    for i in differing[:rows]:
        a = theirs[i] if i < len(theirs) else {}
        b = ours[i] if i < len(ours) else {}
        kind = (a.get("diff_kind") or b.get("diff_kind") or "").removeprefix("DIFF_")
        print(f"! {i:4} {kind:<14} {text(a):<40} | {text(b)}")
    if len(differing) > rows:
        print(f"... {len(differing) - rows} more")
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
