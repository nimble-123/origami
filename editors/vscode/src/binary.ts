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

export interface Resolved {
  binary: string;
  configPath?: string;
}

export class BinaryResolver {
  private cache = new Map<string, Promise<Resolved | undefined>>();

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

  resolve(document: vscode.TextDocument): Promise<Resolved | undefined> {
    const configPath = this.findConfig(document);
    const key = `${vscode.workspace.isTrusted}|${configPath ?? ""}`;
    let hit = this.cache.get(key);
    if (!hit) {
      hit = this.compute(configPath);
      this.cache.set(key, hit);
    }
    return hit;
  }

  private async compute(configPath: string | undefined): Promise<Resolved | undefined> {
    const candidates: string[] = [];
    if (vscode.workspace.isTrusted) {
      const setting = vscode.workspace.getConfiguration("origami").get<string>("path");
      if (setting) {
        // An explicit setting wins without a version check.
        return { binary: setting, configPath };
      }
      if (configPath) {
        candidates.push(process.platform === "win32" ? "origami.exe" : "origami");
      }
    }
    candidates.push(this.bundled());
    for (const binary of candidates) {
      if (!configPath) {
        if (binary === this.bundled() && !fs.existsSync(binary)) {
          continue;
        }
        return { binary, configPath };
      }
      const ok = await this.satisfies(binary, configPath);
      if (ok) {
        return { binary, configPath };
      }
    }
    return undefined;
  }

  private satisfies(binary: string, configPath: string): Promise<boolean> {
    return new Promise((resolve) => {
      execFile(binary, ["--version", "--json", "--config", configPath], { timeout: 2000 }, (err, stdout) => {
        if (err) {
          this.log(`origami: ${binary} --version failed: ${err.message}`);
          resolve(false);
          return;
        }
        try {
          const v = JSON.parse(stdout) as { satisfies_required_version: boolean | null };
          resolve(v.satisfies_required_version !== false);
        } catch {
          resolve(false);
        }
      });
    });
  }
}
