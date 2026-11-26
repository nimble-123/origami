// Binary resolution:
//   1. the machine-scoped setting `origami.path`;
//   2. for documents inside a folder with origami.toml, the first `origami`
//      on PATH that reports satisfies_required_version for that file;
//   3. the bundled binary, checked the same way.
// In untrusted workspaces, steps 1 and 2 are skipped. The binary evaluates
// required_version itself, so this file needs no TOML or semver code.

import * as vscode from "vscode";
import { execFile } from "node:child_process";
import * as path from "node:path";
import * as fs from "node:fs";
import type { Log } from "./format";
import { findOnPath } from "./pathSearch";
import { classifyProbe, Probe } from "./probe";

export interface Resolved {
  binary: string;
  configPath?: string;
}

/** No usable binary: `status` for the status bar, `message` for its tooltip. */
export interface Unresolved {
  status: string;
  message: string;
}

export function isUnresolved(r: Resolved | Unresolved): r is Unresolved {
  return !("binary" in r);
}

const NO_MATCH: Unresolved = {
  status: "origami: version mismatch",
  message: "No origami binary satisfies required_version in origami.toml.",
};

export class BinaryResolver {
  private cache = new Map<string, Promise<Resolved | Unresolved>>();

  constructor(private readonly extensionPath: string, private readonly log: Log) {}

  invalidate(): void {
    this.cache.clear();
  }

  bundled(): string {
    const exe = process.platform === "win32" ? "origami.exe" : "origami";
    return path.join(this.extensionPath, "bin", exe);
  }

  /** Nearest origami.toml for a file document, if any. */
  private findConfig(document: vscode.TextDocument): string | undefined {
    if (document.uri.scheme !== "file") {
      return undefined;
    }
    const folder = vscode.workspace.getWorkspaceFolder(document.uri);
    let dir = path.dirname(document.uri.fsPath);
    const stop = folder?.uri.fsPath;
    for (;;) {
      const candidate = path.join(dir, "origami.toml");
      if (fs.existsSync(candidate)) {
        return candidate;
      }
      if (fs.existsSync(path.join(dir, ".git")) || dir === stop) {
        return undefined;
      }
      const parent = path.dirname(dir);
      if (parent === dir) {
        return undefined;
      }
      dir = parent;
    }
  }

  resolve(document: vscode.TextDocument): Promise<Resolved | Unresolved> {
    const configPath = this.findConfig(document);
    const key = `${vscode.workspace.isTrusted}|${configPath ?? ""}`;
    let hit = this.cache.get(key);
    if (!hit) {
      const pending: Promise<Resolved | Unresolved> = this.compute(configPath).then(
        ({ result, transient }) => {
          // A timeout or spawn error says nothing about the binary: ask again next time.
          if (transient && this.cache.get(key) === pending) {
            this.cache.delete(key);
          }
          return result;
        },
      );
      hit = pending;
      this.cache.set(key, hit);
    }
    return hit;
  }

  private async compute(
    configPath: string | undefined,
  ): Promise<{ result: Resolved | Unresolved; transient: boolean }> {
    const candidates: string[] = [];
    if (vscode.workspace.isTrusted) {
      const setting = vscode.workspace.getConfiguration("origami").get<string>("path");
      if (setting) {
        // An explicit setting wins without a version check.
        return { result: { binary: setting, configPath }, transient: false };
      }
      if (configPath) {
        // By absolute path: a bare name would let Windows pick an origami.exe
        // from the workspace folder, which is the working directory.
        const onPath = findOnPath("origami");
        if (onPath) {
          candidates.push(onPath);
        }
      }
    }
    candidates.push(this.bundled());
    let transient = false;
    for (const binary of candidates) {
      if (!configPath) {
        if (binary === this.bundled() && !fs.existsSync(binary)) {
          continue;
        }
        return { result: { binary, configPath }, transient };
      }
      const probe = await this.probe(binary, configPath);
      switch (probe.kind) {
        case "satisfied":
          return { result: { binary, configPath }, transient };
        case "config":
          // The same for every binary: report it instead of a version mismatch.
          this.log(`origami: ${configPath}: ${probe.message}`);
          return {
            result: {
              status: "origami: invalid origami.toml",
              message: `${path.basename(configPath)}: ${probe.message}`,
            },
            transient,
          };
        case "transient":
          transient = true;
          break;
        case "unsatisfied":
          break;
      }
    }
    return { result: NO_MATCH, transient };
  }

  private probe(binary: string, configPath: string): Promise<Probe> {
    return new Promise((resolve) => {
      execFile(binary, ["--version", "--json", "--config", configPath], { timeout: 2000 }, (err, stdout, stderr) => {
        if (err) {
          this.log(`origami: ${binary} --version failed: ${err.message}`);
        }
        resolve(classifyProbe(err, stdout, stderr));
      });
    });
  }
}
