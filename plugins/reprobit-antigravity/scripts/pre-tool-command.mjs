#!/usr/bin/env node
// PreToolUse guard for run_command. Reads the tool call JSON on stdin and prints
// {decision, reason}. Denies `git add` of reference/, .reprobit/ and disc images.
import fs from "node:fs";

const DISC_EXT = /\.(iso|cue|chd|gcm|rvz|wia|wbfs|ciso|gcz|mdf|nsp|xci|z64|n64|v64)$/i;

function respond(obj) {
  process.stdout.write(`${JSON.stringify(obj)}\n`);
  process.exit(0);
}

function readInput() {
  try {
    return JSON.parse(fs.readFileSync(0, "utf8") || "{}");
  } catch {
    return {};
  }
}

function tokenize(segment) {
  const out = [];
  const re = /"([^"]*)"|'([^']*)'|(\S+)/g;
  for (let m = re.exec(segment); m; m = re.exec(segment)) out.push(m[1] ?? m[2] ?? m[3]);
  return out;
}

export function forbiddenPath(raw) {
  let p = raw.replace(/\\/g, "/");
  while (p.startsWith("./") || p.startsWith("../")) p = p.slice(p.indexOf("/") + 1);
  const parts = p.split("/").filter(Boolean);
  if (parts.includes(".reprobit")) return "`.reprobit/` holds machine-local build state";
  if (parts[0] === "reference") return "`reference/` holds original binaries";
  if (DISC_EXT.test(p)) return "disc images and dumps are copyrighted";
  return null;
}

export function check(cmd) {
  for (const segment of cmd.split(/&&|\|\||[;|\n]/)) {
    const tokens = tokenize(segment);
    const git = tokens.indexOf("git");
    if (git < 0) continue;
    const add = tokens.indexOf("add", git + 1);
    if (add < 0) continue;
    for (const arg of tokens.slice(add + 1)) {
      if (arg.startsWith("-")) continue;
      const why = forbiddenPath(arg);
      if (why) {
        return {
          decision: "deny",
          reason: `Do not stage ${arg}: ${why}. Stage source files by explicit path instead.`,
        };
      }
    }
  }
  return { decision: "allow" };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const args = readInput()?.toolCall?.args || {};
  const cmd = typeof args.CommandLine === "string" ? args.CommandLine : typeof args.command === "string" ? args.command : "";
  respond(cmd.trim() ? check(cmd) : { decision: "allow" });
}
