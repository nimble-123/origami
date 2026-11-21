// Minimal edits between two texts, independent of the VS Code API so it can be
// unit-tested with node:test.
//
// Line-level diff via jsdiff; each changed hunk becomes one replacement. A
// replacement is trimmed to its common prefix and suffix, so a whitespace
// change inside a line touches only that part of the line.

import { diffLines } from "diff";

export interface Position {
  line: number;
  character: number;
}

export interface Replacement {
  start: Position;
  end: Position;
  text: string;
}

/** Position of a UTF-16 offset in `text`. */
function positionAt(lineStarts: number[], offset: number): Position {
  let lo = 0;
  let hi = lineStarts.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (lineStarts[mid] <= offset) {
      lo = mid;
    } else {
      hi = mid - 1;
    }
  }
  return { line: lo, character: offset - lineStarts[lo] };
}

function lineStartsOf(text: string): number[] {
  const starts = [0];
  for (let i = 0; i < text.length; i++) {
    if (text.charCodeAt(i) === 10) {
      starts.push(i + 1);
    }
  }
  return starts;
}

export function computeEdits(oldText: string, newText: string): Replacement[] {
  if (oldText === newText) {
    return [];
  }
  const starts = lineStartsOf(oldText);
  const parts = diffLines(oldText, newText);
  const edits: Replacement[] = [];
  let offset = 0; // offset in oldText
  let k = 0;
  while (k < parts.length) {
    const part = parts[k];
    if (!part.added && !part.removed) {
      offset += part.value.length;
      k++;
      continue;
    }
    // A hunk: consecutive removed/added parts.
    let removed = "";
    let added = "";
    while (k < parts.length && (parts[k].added || parts[k].removed)) {
      if (parts[k].removed) {
        removed += parts[k].value;
      } else {
        added += parts[k].value;
      }
      k++;
    }
    // Trim the common prefix and suffix of the hunk.
    let p = 0;
    while (p < removed.length && p < added.length && removed[p] === added[p]) {
      p++;
    }
    let s = 0;
    while (
      s < removed.length - p &&
      s < added.length - p &&
      removed[removed.length - 1 - s] === added[added.length - 1 - s]
    ) {
      s++;
    }
    const from = offset + p;
    const to = offset + removed.length - s;
    edits.push({
      start: positionAt(starts, from),
      end: positionAt(starts, to),
      text: added.slice(p, added.length - s),
    });
    offset += removed.length;
  }
  return edits;
}

/** Applies replacements (in order, non-overlapping) to a text; for tests. */
export function applyEdits(text: string, edits: Replacement[]): string {
  const starts = lineStartsOf(text);
  const toOffset = (p: Position) => starts[p.line] + p.character;
  let out = "";
  let last = 0;
  for (const e of edits) {
    out += text.slice(last, toOffset(e.start)) + e.text;
    last = toOffset(e.end);
  }
  return out + text.slice(last);
}
