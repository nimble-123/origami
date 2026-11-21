import { test } from "node:test";
import * as assert from "node:assert/strict";
import { applyEdits, computeEdits } from "../src/edits";
import { parseRun, toEditorRange } from "../src/diagnosticsModel";

const cases: Array<[string, string]> = [
  ["a\nb\nc\n", "a\nb\nc\n"],
  ["DATA  x TYPE i.\n", "DATA x TYPE i.\n"],
  ["IF x = 1.\ndo( ).\nENDIF.\n", "IF x = 1.\n  do( ).\nENDIF.\n"],
  ["a = 1. b = 2.\n", "a = 1.\nb = 2.\n"],
  ["x\n\n\ny\n", "x\n\ny\n"],
  ["ä😀 = 1.\nz = 2.\n", "ä😀 = 1.\n  z = 2.\n"],
  ["no final newline", "no final newline\n"],
  ["", "x\n"],
];

test("edits transform old text into new text", () => {
  for (const [a, b] of cases) {
    assert.equal(applyEdits(a, computeEdits(a, b)), b, JSON.stringify([a, b]));
  }
});

test("identical texts produce no edits", () => {
  assert.deepEqual(computeEdits("same\n", "same\n"), []);
});

test("an indentation change edits only the start of the line", () => {
  const edits = computeEdits("IF x.\ndo( ).\nENDIF.\n", "IF x.\n  do( ).\nENDIF.\n");
  assert.equal(edits.length, 1);
  assert.deepEqual(edits[0].start, { line: 1, character: 0 });
  assert.deepEqual(edits[0].end, { line: 1, character: 0 });
  assert.equal(edits[0].text, "  ");
});

test("diagnostics JSON is parsed and positions become 0-based", () => {
  const run = parseRun(
    JSON.stringify({
      schema: 1,
      version: "0.1.0",
      style_edition: "2026",
      truncated: false,
      files: [
        {
          path: "<stdin>",
          language: "abap",
          changed: false,
          status: "error",
          diagnostics: [
            {
              code: "E004",
              severity: "error",
              message: "Missing ENDIF",
              range: { start: { line: 3, column: 5, offset: 20 }, end: { line: 3, column: 7, offset: 22 } },
            },
          ],
        },
      ],
    }),
  );
  assert.ok(run);
  const r = toEditorRange(run.files[0].diagnostics[0]);
  assert.deepEqual(r, { startLine: 2, startCharacter: 4, endLine: 2, endCharacter: 6 });
  assert.equal(parseRun("not json"), undefined);
});
