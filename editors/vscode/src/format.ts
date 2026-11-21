// One formatting request: spawn origami with the document text on stdin.
//
// Exit 0 or 5: apply the edits. Exit 2: structural error, no edits, show the
// diagnostics. Any other exit, a timeout or a cancellation: no edits.

import * as vscode from "vscode";
import { spawn } from "node:child_process";
import * as os from "node:os";
import { computeEdits } from "./edits";
import { JsonRun, parseRun } from "./diagnosticsModel";
import { Resolved } from "./binary";
import { targetFor } from "./uri";

export interface Outcome {
  edits: vscode.TextEdit[];
  run?: JsonRun;
  status: "ok" | "error" | "timeout" | "failed";
}

const TIMEOUT_MS = 2000;

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

export function formatDocument(
  document: vscode.TextDocument,
  resolved: Resolved,
  token: vscode.CancellationToken,
  log: (m: string) => void,
  ranges: readonly vscode.Range[] = [],
): Promise<Outcome> {
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
  log(`${resolved.binary} ${args.join(" ")}`);

  return new Promise((resolve) => {
    const child = spawn(resolved.binary, args, { cwd });
    let stdout = "";
    let stderr = "";
    let settled = false;
    const finish = (o: Outcome) => {
      if (!settled) {
        settled = true;
        clearTimeout(timer);
        resolve(o);
      }
    };
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      finish({ edits: [], status: "timeout" });
    }, TIMEOUT_MS);
    token.onCancellationRequested(() => {
      child.kill("SIGKILL");
      finish({ edits: [], status: "failed" });
    });
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (d: string) => (stdout += d));
    child.stderr.on("data", (d: string) => (stderr += d));
    child.on("error", (err) => {
      log(`origami: cannot start ${resolved.binary}: ${err.message}`);
      finish({ edits: [], status: "failed" });
    });
    child.on("close", (code) => {
      if (stderr) {
        log(stderr.trim());
      }
      const run = parseRun(stderr);
      if (code === 0 || code === 5) {
        const original = document.getText();
        const changed = run?.files[0]?.changed ?? stdout !== original;
        if (!changed || stdout.length === 0) {
          finish({ edits: [], run, status: "ok" });
          return;
        }
        const edits = computeEdits(original, stdout).map(
          (e) =>
            new vscode.TextEdit(
              new vscode.Range(e.start.line, e.start.character, e.end.line, e.end.character),
              e.text,
            ),
        );
        finish({ edits, run, status: "ok" });
      } else if (code === 2) {
        finish({ edits: [], run, status: "error" });
      } else {
        finish({ edits: [], run, status: "failed" });
      }
    });
    child.stdin.end(document.getText(), "utf8");
  });
}
