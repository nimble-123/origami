// Binary resolution:
//   1. the machine-scoped setting `origami.path`;
//   2. for documents inside a folder with origami.toml, the first `origami`
//      on PATH that reports satisfies_required_version for that file;
//   3. the bundled binary, checked the same way;
//   4. an `origami` on PATH, if the extension ships no binary for this platform.
// In untrusted workspaces, steps 1, 2 and 4 are skipped. The binary evaluates
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

const NOT_FOUND: Unresolved = {
  status: "origami: not found",
  message:
    "No origami binary found: this extension ships none for your platform and none is on PATH. Install origami or set origami.path.",
};

const NOT_FOUND_UNTRUSTED: Unresolved = {
  status: "origami: not found",
  message:
    "No origami binary found: this extension ships none for your platform, and PATH is not used in untrusted workspaces. Trust the workspace, or install a build of the extension with a bundled binary.",
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
    const trusted = vscode.workspace.isTrusted;
    if (trusted) {
      const setting = vscode.workspace.getConfiguration("origami").get<string>("path");
      if (setting) {
        // An explicit setting wins without a version check.
        return { result: { binary: setting, configPath }, transient: false };
      }
    }
    // By absolute path: a bare name would let Windows pick an origami.exe
    // from the workspace folder, which is the working directory.
    const onPath = trusted ? findOnPath("origami") : undefined;
    const bundled = fs.existsSync(this.bundled()) ? this.bundled() : undefined;
    // With an origami.toml, PATH is tried first for its required_version.
    // Without one there is nothing to check: the bundled binary, and PATH
    // only if the extension ships none for this platform.
    const candidates = (configPath ? [onPath, bundled] : [bundled ?? onPath]).filter(
      (b): b is string => b !== undefined,
    );
    if (candidates.length === 0) {
      return { result: trusted ? NOT_FOUND : NOT_FOUND_UNTRUSTED, transient: false };
    }
    let transient = false;
    let configError: string | undefined;
    for (const binary of candidates) {
      if (!configPath) {
        return { result: { binary, configPath }, transient };
      }
      const probe = await this.probe(binary, configPath);
      switch (probe.kind) {
        case "satisfied":
          return { result: { binary, configPath }, transient };
        case "config":
          // Exit 3 is also a usage error: an older binary on PATH may not
          // know the probe's flags, so the next candidate still gets a try.
          this.log(`origami: ${binary}: ${configPath}: ${probe.message}`);
          configError ??= probe.message;
          break;
        case "transient":
          transient = true;
          break;
        case "unsatisfied":
          break;
      }
    }
    if (configError !== undefined) {
      return {
        result: {
          status: "origami: invalid origami.toml",
          message: `${path.basename(configPath ?? "origami.toml")}: ${configError}`,
        },
        transient,
      };
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
