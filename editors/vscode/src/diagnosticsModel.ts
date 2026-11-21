// The JSON diagnostics written by `origami --diagnostics=json` (schema 1),
// and their conversion to 0-based editor positions. Independent of the VS
// Code API so it can be unit-tested.

export interface JsonPosition {
  line: number; // 1-based
  column: number; // 1-based, UTF-16 code units
  offset: number;
}

export interface JsonDiagnostic {
  code: string;
  severity: "info" | "warning" | "error" | "internal";
  message: string;
  range: { start: JsonPosition; end: JsonPosition };
}

export interface JsonFile {
  path: string;
  language: string | null;
  changed: boolean;
  status: "formatted" | "unchanged" | "error" | "skipped";
  diagnostics: JsonDiagnostic[];
}

export interface JsonRun {
  schema: number;
  version: string;
  style_edition: string;
  truncated: boolean;
  files: JsonFile[];
}

export interface EditorRange {
  startLine: number;
  startCharacter: number;
  endLine: number;
  endCharacter: number;
}

/** 1-based line/column to 0-based line/character. */
export function toEditorRange(d: JsonDiagnostic): EditorRange {
  return {
    startLine: Math.max(0, d.range.start.line - 1),
    startCharacter: Math.max(0, d.range.start.column - 1),
    endLine: Math.max(0, d.range.end.line - 1),
    endCharacter: Math.max(0, d.range.end.column - 1),
  };
}

/** Parses stderr; returns undefined when it is not a schema-1 document. */
export function parseRun(stderr: string): JsonRun | undefined {
  const text = stderr.trim();
  if (!text.startsWith("{")) {
    return undefined;
  }
  try {
    const run = JSON.parse(text) as JsonRun;
    return run.schema === 1 ? run : undefined;
  } catch {
    return undefined;
  }
}
