"""PreToolUse guard: block git commands that stage reference/, .reprobit/ or disc images.

Reads the hook JSON on stdin. Prints a PreToolUse deny decision and exits 0 to block;
prints nothing and exits 0 to allow. Standard library only.
"""

import json
import os
import re
import shlex
import subprocess
import sys

PROTECTED_DIRS = {"reference", ".reprobit"}
PROTECTED_EXTS = (".iso", ".cue", ".chd", ".gcm", ".rvz", ".wia", ".wbfs", ".ciso", ".gcz", ".mdf", ".nsp", ".xci", ".z64", ".n64", ".v64")
BROAD_ADD = {".", "*", ":/", ":/*", "-A", "--all", "-u", "--update", "--no-ignore-removal"}
SEPARATORS = re.compile(r"\|\||&&|;|\||\n|&")


def protected(path):
    p = path.replace("\\", "/")
    if p.startswith(":"):
        p = re.sub(r"^:(\([^)]*\)|[^a-zA-Z0-9./_-]*)", "", p)
    parts = [x for x in p.split("/") if x not in ("", ".")]
    if not parts:
        return False
    if any(x in PROTECTED_DIRS for x in parts):
        return True
    return parts[-1].lower().endswith(PROTECTED_EXTS)


def git_lines(cwd, *args):
    try:
        out = subprocess.run(
            ["git", "-C", cwd, *args], capture_output=True, text=True, timeout=5
        )
    except (OSError, subprocess.SubprocessError):
        return []
    return out.stdout.split("\0") if out.returncode == 0 else []


def tracked_or_new_hits(cwd, new):
    args = ["ls-files", "-z", "--modified"]
    if new:
        args += ["--others", "--exclude-standard"]
    return [f for f in git_lines(cwd, *args) if f and protected(f)]


def staged_hits(cwd):
    return [f for f in git_lines(cwd, "diff", "--cached", "--name-only", "-z") if f and protected(f)]


def check_segment(tokens, cwd):
    """Return a reason string when the segment stages protected files."""
    i = 0
    while i < len(tokens) and "=" in tokens[i] and not tokens[i].startswith("-"):
        i += 1  # leading VAR=value
    if i >= len(tokens) or os.path.basename(tokens[i]) != "git":
        return None
    i += 1
    while i < len(tokens) and tokens[i].startswith("-"):  # git global options
        if tokens[i] == "-C" and i + 1 < len(tokens):
            cwd = os.path.join(cwd, tokens[i + 1])
            i += 1
        elif tokens[i] in ("-c", "--git-dir", "--work-tree", "--namespace"):
            i += 1
        i += 1
    if i >= len(tokens):
        return None
    sub, args = tokens[i], tokens[i + 1:]
    if "--" in args:
        k = args.index("--")
        flags, paths = args[:k], args[k + 1:]
    else:
        flags = [a for a in args if a.startswith("-")]
        paths = [a for a in args if not a.startswith("-")]
    if sub == "add":
        named = [p for p in paths if protected(p)]
        if named:
            return "git add names " + ", ".join(named)
        broad = any(f in BROAD_ADD for f in flags) or any(p in BROAD_ADD for p in paths)
        if broad:
            update_only = bool({"-u", "--update"} & set(flags)) and not {"-A", "--all"} & set(flags)
            hits = tracked_or_new_hits(cwd, new=not update_only)
            if hits:
                return "git add would stage " + ", ".join(hits[:5])
        return None
    if sub == "commit":
        named = [p for p in paths if protected(p)]
        all_flag = any(
            f in ("--all", "-a") or (re.fullmatch(r"-[A-Za-z]+", f) and "a" in f[1:] and not f.startswith("--"))
            for f in flags
        )
        hits = staged_hits(cwd) if not named else []
        if all_flag:
            hits += tracked_or_new_hits(cwd, new=False)
        if named:
            return "git commit names " + ", ".join(named)
        if hits:
            return "git commit would include " + ", ".join(hits[:5])
    return None


def main():
    try:
        data = json.load(sys.stdin)
    except ValueError:
        return 0
    if data.get("tool_name") != "Bash":
        return 0
    command = (data.get("tool_input") or {}).get("command") or ""
    cwd = data.get("cwd") or os.getcwd()
    for segment in SEPARATORS.split(command):
        try:
            tokens = shlex.split(segment)
        except ValueError:
            tokens = segment.split()
        reason = check_segment(tokens, cwd)
        if reason:
            print(json.dumps({"hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": (
                    "ReproBit guard: " + reason + ". reference/, .reprobit/ and disc images "
                    "stay out of git. Stage explicit source paths instead."
                ),
            }}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
