// One formatting request: spawn origami with a text on stdin.
//
// Exit 0 or 5: the formatted text (or none if nothing changes). Exit 2: structural error, no edits, show the
// diagnostics. Any other exit, a timeout or a cancellation: no edits.

import * as vscode from "vscode";
import { spawn } from "node:child_process";
import * as os from "node:os";
import { computeEdits } from "./edits";
import { JsonRun, parseRun } from "./diagnosticsModel";
import { Resolved } from "./binary";
import { targetFor } from "./uri";

/** Logs a line to the Output channel; `verbose` lines only with `origami.trace`. */
export type Log = (message: string, verbose?: boolean) => void;

export type Status = "ok" | "error" | "timeout" | "failed";

export interface Outcome {
  edits: vscode.TextEdit[];
  run?: JsonRun;
  status: Status;
}

export interface TextOutcome {
  /** The formatted text; undefined if nothing changes or on failure. */
  text?: string;
  run?: JsonRun;
  status: Status;
}

/** Timeout of a formatting request from the editor. */
export const FORMAT_TIMEOUT_MS = 2000;

/** 1-based inclusive line ranges for `--lines`; a range that ends at
 * column 0 of a later line does not include that line. */
export function linesArgs(ranges: readonly vscode.Range[]): string[] {
  return ranges.flatMap((r) => {
    const start = r.start.line + 1;
    let end = r.end.line + 1;
    if (r.end.character === 0 && r.end.line > r.start.line) {
      end -= 1;
    }
    return ["--lines", `${start}:${end}`];
  });
}

export async function formatDocument(
  document: vscode.TextDocument,
  resolved: Resolved,
  token: vscode.CancellationToken,
  log: Log,
  ranges: readonly vscode.Range[] = [],
): Promise<Outcome> {
  const original = document.getText();
  const o = await formatText(original, document, resolved, token, log, ranges, FORMAT_TIMEOUT_MS);
  const edits =
    o.text === undefined
      ? []
      : computeEdits(original, o.text).map(
          (e) =>
            new vscode.TextEdit(
              new vscode.Range(e.start.line, e.start.character, e.end.line, e.end.character),
              e.text,
            ),
        );
  return { edits, run: o.run, status: o.status };
}

/** Formats `text` as the content of `document` (its name, language and
 * configuration). */
export function formatText(
  text: string,
  document: vscode.TextDocument,
  resolved: Resolved,
  token: vscode.CancellationToken,
  log: Log,
  ranges: readonly vscode.Range[],
  timeoutMs: number,
): Promise<TextOutcome> {
  const target = targetFor(document);
  const args = ["--stdin", "--stdin-filepath", target.name, "--diagnostics=json", ...linesArgs(ranges)];
  if (target.language) {
    args.push("--language", target.language);
  }
  if (!target.useConfig) {
    args.push("--no-config");
  } else if (resolved.configPath) {
    args.push("--config", resolved.configPath);
  }
  const folder = vscode.workspace.getWorkspaceFolder(document.uri);
  const cwd = folder?.uri.scheme === "file" ? folder.uri.fsPath : os.tmpdir();
  log(`${resolved.binary} ${args.join(" ")}`, true);

  return new Promise((resolve) => {
    const child = spawn(resolved.binary, args, { cwd });
    let stdout = "";
    let stderr = "";
    let settled = false;
    const finish = (o: TextOutcome) => {
      if (!settled) {
        settled = true;
        clearTimeout(timer);
        resolve(o);
      }
    };
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      finish({ status: "timeout" });
    }, timeoutMs);
    token.onCancellationRequested(() => {
      child.kill("SIGKILL");
      finish({ status: "failed" });
    });
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (d: string) => (stdout += d));
    child.stderr.on("data", (d: string) => (stderr += d));
    child.on("error", (err) => {
      log(`origami: cannot start ${resolved.binary}: ${err.message}`);
      finish({ status: "failed" });
    });
    child.on("close", (code) => {
      const run = parseRun(stderr);
      if (stderr) {
        // Routine diagnostics of a finished run only with `origami.trace`;
        // the stderr of a failed run is the only record of why.
        log(stderr.trim(), code === 0 || code === 2 || code === 5);
      }
      if (code === 0 || code === 5) {
        const changed = run?.files[0]?.changed ?? stdout !== text;
        if (!changed || stdout.length === 0) {
          finish({ run, status: "ok" });
          return;
        }
        finish({ text: stdout, run, status: "ok" });
      } else if (code === 2) {
        finish({ run, status: "error" });
      } else {
        finish({ run, status: "failed" });
      }
    });
    // The binary can exit before it has read all of stdin (an unknown flag, a
    // crash); the write then fails with EPIPE. 'close' decides the outcome.
    child.stdin.on("error", (err: NodeJS.ErrnoException) => {
      if (err.code !== "EPIPE") {
        log(`origami: cannot write to ${resolved.binary}: ${err.message}`);
      }
    });
    child.stdin.end(text, "utf8");
  });
}
