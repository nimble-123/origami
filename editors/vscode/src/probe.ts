// Reading the result of `origami --version --json --config FILE`. No VS Code
// imports, so the node test runner can load it.

export type Probe =
  /** The binary satisfies required_version (or the file sets none). */
  | { kind: "satisfied" }
  /** The binary does not satisfy required_version, or cannot answer the question. */
  | { kind: "unsatisfied" }
  /** Timeout or spawn error: says nothing about the binary, so do not remember it. */
  | { kind: "transient" }
  /** Exit 3: origami.toml is invalid (or required_version is not a valid range). */
  | { kind: "config"; message: string };

/** The part of `execFile`'s error that matters here. */
export interface ProbeError {
  /** Exit status (number) or system error such as `ENOENT` (string). */
  code?: number | string | null;
  /** True if the process was killed, for example by the timeout. */
  killed?: boolean;
  message: string;
}

export function classifyProbe(err: ProbeError | null, stdout: string, stderr: string): Probe {
  if (err) {
    if (err.killed || typeof err.code === "string") {
      return { kind: "transient" };
    }
    if (err.code === 3) {
      const message = stderr.trim().replace(/^origami:\s*/, "");
      return { kind: "config", message: message || err.message };
    }
    return { kind: "unsatisfied" };
  }
  try {
    const v = JSON.parse(stdout) as { satisfies_required_version: boolean | null };
    return { kind: v.satisfies_required_version === false ? "unsatisfied" : "satisfied" };
  } catch {
    return { kind: "unsatisfied" };
  }
}
