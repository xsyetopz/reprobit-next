import {
  DEFAULT_MAX_BYTES,
  DEFAULT_MAX_LINES,
  type ExtensionAPI,
  formatSize,
  isToolCallEventType,
  truncateTail,
} from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

/** Disc images and ROM dumps, which belong to the user's own copy and never to a repository. */
const DUMP = /\.(iso|cue|chd|gcm|rvz|wia|wbfs|ciso|gcz|mdf|nsp|xci|z64|n64|v64)$/i;

/** Paths a matching decompilation must never commit: references, rbit state, and dumps. */
export function protectedPath(path: string): boolean {
  const parts = segments(path);
  return parts.includes("reference") || parts.includes(".reprobit") || DUMP.test(path);
}

function segments(path: string): string[] {
  return path.replace(/\\/g, "/").split("/");
}

/** The git subcommand and its position, skipping global options and the values `-C` and `-c` take. */
function gitVerb(words: string[], git: number): number {
  for (let at = git + 1; at < words.length; at++) {
    if (words[at] === "-C" || words[at] === "-c") at++;
    else if (!words[at].startsWith("-")) return at;
  }
  return -1;
}

/**
 * Names the protected path a shell command stages, or returns `undefined`.
 *
 * Only explicit pathspecs are checked; `git add -A` and `git add .` stage what `.gitignore`
 * allows, which a hook cannot see without running git.
 */
export function stagedProtectedPath(command: string): string | undefined {
  for (const part of command.split(/&&|\|\||;|\|/)) {
    const words = part.trim().split(/\s+/);
    const git = words.indexOf("git");
    if (git < 0) continue;
    const verb = gitVerb(words, git);
    if (verb < 0 || (words[verb] !== "add" && words[verb] !== "stage")) continue;
    const paths = words.slice(verb + 1).filter((word) => !word.startsWith("-"));
    const hit = paths.map((word) => word.replace(/^["']|["']$/g, "")).find(protectedPath);
    if (hit) return hit;
  }
  return undefined;
}

export default function reprobit(pi: ExtensionAPI) {
  pi.registerTool({
    name: "rbit",
    label: "rbit",
    description:
      "Run the ReproBit CLI with --json, such as args [\"status\"], [\"diff\"], [\"verify\"] or " +
      "[\"grind\", \"--step\", \"s\", \"--symbol\", \"f\", \"--reference-object\", \"o\"]. Exit 1 is " +
      "a negative result, not a failure. Output keeps its last 50 KB or 2000 lines.",
    parameters: Type.Object({
      args: Type.Array(Type.String(), { description: "Arguments after `rbit --json`" }),
      cwd: Type.Optional(Type.String({ description: "Project directory; defaults to the session's" })),
    }),
    async execute(_id, params, signal, _onUpdate, ctx) {
      const result = await pi.exec("rbit", ["--json", ...params.args], {
        signal,
        cwd: params.cwd ?? ctx.cwd,
      });
      const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
      const cut = truncateTail(output, { maxBytes: DEFAULT_MAX_BYTES, maxLines: DEFAULT_MAX_LINES });
      let text = `exit ${result.code}\n${cut.content}`;
      if (cut.truncated) {
        const file = join(mkdtempSync(join(tmpdir(), "rbit-")), "output.txt");
        writeFileSync(file, output);
        text += `\n[Kept the last ${formatSize(cut.outputBytes)} of ${formatSize(cut.totalBytes)}; full output: ${file}]`;
      }
      return { content: [{ type: "text", text }], details: { code: result.code, truncated: cut.truncated } };
    },
  });

  pi.on("tool_call", async (event) => {
    if (isToolCallEventType("bash", event)) {
      const hit = stagedProtectedPath(event.input.command);
      if (hit) {
        return { block: true, reason: `ReproBit: ${hit} is a reference, rbit state or a disc image; keep it out of git.` };
      }
    }
    if (isToolCallEventType("write", event) || isToolCallEventType("edit", event)) {
      if (segments(event.input.path).includes("reference")) {
        return { block: true, reason: "ReproBit: reference/ holds the user's original binaries; never write there." };
      }
    }
    return undefined;
  });
}
