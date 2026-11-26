#!/bin/sh
# Builds reference/ for this example: the same source compiled in the
# declaration state the "original" was built in (a shape prelude with one
# class and ten functions). The binaries are generated, never committed.
# `rbit status` then checks reference/grind.exe against the pinned digest.
set -eu
cd "$(dirname "$0")"
rbit="${RBIT:-rbit}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cp transform.cpp reprobit.lock "$work/" 2>/dev/null || cp transform.cpp "$work/"
sed 's/^id = "compile.transform"$/&\
prelude = { family = "shape", classes = 1, functions = 10 }/' reprobit.toml >"$work/reprobit.toml"
"$rbit" --quiet build "$work" >/dev/null
mkdir -p reference
cp "$work/.reprobit/stage/build/obj/transform.obj" reference/reference.obj
cp "$work/.reprobit/stage/build/grind.exe" reference/grind.exe
echo "prepared reference/reference.obj and reference/grind.exe"
