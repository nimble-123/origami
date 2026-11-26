// Finding an executable on PATH. The extension spawns the result by absolute
// path: with a bare name, Windows searches the child's working directory (the
// workspace folder) before PATH, so an `origami.exe` in a cloned repository
// would run instead. No VS Code imports, so the node test runner can load it.

import * as fs from "node:fs";
import * as path from "node:path";

export interface FindOptions {
  env: NodeJS.ProcessEnv;
  platform: NodeJS.Platform;
  /** True if the path is an existing file that can be run. */
  isFile: (p: string) => boolean;
}

export function isRunnableFile(p: string): boolean {
  try {
    if (!fs.statSync(p).isFile()) {
      return false;
    }
    if (process.platform !== "win32") {
      fs.accessSync(p, fs.constants.X_OK);
    }
    return true;
  } catch {
    return false;
  }
}

/** The absolute path of the first `name` on PATH, or undefined. Relative and
 * empty PATH entries are skipped, since they mean the current directory. On
 * Windows the extensions of PATHEXT are tried, limited to .exe and .com,
 * which can be spawned without a shell. */
export function findOnPath(
  name: string,
  { env, platform, isFile }: FindOptions = {
    env: process.env,
    platform: process.platform,
    isFile: isRunnableFile,
  },
): string | undefined {
  const win = platform === "win32";
  const p = win ? path.win32 : path.posix;
  const dirs = (env.PATH ?? env.Path ?? "").split(win ? ";" : ":");
  const names = win
    ? (env.PATHEXT ?? ".COM;.EXE;.BAT;.CMD")
        .split(";")
        .filter((e) => /^\.(exe|com)$/i.test(e))
        .map((e) => name + e)
    : [name];
  for (const dir of dirs) {
    const unquoted = dir.replace(/^"(.*)"$/, "$1");
    if (!unquoted || !p.isAbsolute(unquoted)) {
      continue;
    }
    for (const n of names) {
      const candidate = p.join(unquoted, n);
      if (isFile(candidate)) {
        return candidate;
      }
    }
  }
  return undefined;
}
