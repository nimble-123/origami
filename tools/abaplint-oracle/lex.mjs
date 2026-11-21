// Independent lexer oracle for origami CI (not shipped).
//
// Reads file paths from stdin, one per line, and prints NDJSON to stdout, one
// line per file: {"path", "tokens": [{"type", "text", "row", "col"}]}.
// abaplint's punctuation and operator token types encode adjacent whitespace
// (for example `WParenLeft` vs `ParenLeft`); word tokens do not.

import { readFileSync } from "node:fs";
import { basename } from "node:path";
import { MemoryFile, Registry } from "@abaplint/core";

const input = readFileSync(0, "utf8");
const paths = input.split(/\r?\n/).filter((l) => l.length > 0);

for (const path of paths) {
  const text = readFileSync(path, "utf8");
  // abaplint derives the object from the abapGit file name.
  const name = basename(path).replace(/^.*__/, "");
  const reg = new Registry();
  reg.addFile(new MemoryFile(name, text));
  reg.parse();
  const tokens = [];
  for (const obj of reg.getObjects()) {
    for (const file of obj.getABAPFiles?.() ?? []) {
      for (const t of file.getTokens()) {
        tokens.push({
          type: t.constructor.name,
          text: t.getStr(),
          row: t.getRow(),
          col: t.getCol(),
        });
      }
    }
  }
  process.stdout.write(JSON.stringify({ path, tokens }) + "\n");
}
