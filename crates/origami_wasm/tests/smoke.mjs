// Smoke test of the origami.wasm C ABI (alloc -> review -> dealloc), the way
// site/worker.js calls it. Plain Node, no dependencies.
//   node crates/origami_wasm/tests/smoke.mjs [path/to/origami_wasm.wasm]
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const wasmPath =
  process.argv[2] ??
  new URL("../../../target/wasm32-unknown-unknown/release/origami_wasm.wasm", import.meta.url);
const { instance } = await WebAssembly.instantiate(await readFile(wasmPath), {});
const wasm = instance.exports;
for (const name of ["memory", "alloc", "dealloc", "review", "result_len"]) {
  assert.ok(name in wasm, `missing export: ${name}`);
}

function review(source, { cds = false, width = 120, indent = 2, lower = false, compact = false } = {}) {
  const bytes = new TextEncoder().encode(source);
  const ptr = wasm.alloc(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  const res = wasm.review(ptr, bytes.length, cds ? 1 : 0, width, indent, lower ? 1 : 0, compact ? 1 : 0);
  const len = wasm.result_len();
  const json = new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, res, len));
  wasm.dealloc(res, len);
  return JSON.parse(json);
}

// ABAP: two statements on one line are split, with a note naming the rule.
let r = review("lv_a = 1. lv_b = 2.\n");
assert.equal(r.status, "formatted");
assert.equal(r.output, "lv_a = 1.\nlv_b = 2.\n");
assert.equal(r.notes[0].rule, "ABAP-STMT-001");

// Structural error: no output, a diagnostic.
r = review("lv_a = 1\n");
assert.equal(r.status, "error");
assert.equal(r.output, null);
assert.equal(r.diagnostics[0].code, "E003");

// CDS.
r = review("define view entity Z as select from t { key a, b }\n", { cds: true });
assert.equal(r.language, "ddls");

// Empty input, then many round trips: every buffer is freed, memory stays flat.
assert.equal(review("").status, "unchanged");
review("lv_a = 1. lv_b = 2.\n");
const before = wasm.memory.buffer.byteLength;
for (let i = 0; i < 2000; i++) review("lv_a = 1. lv_b = 2.\n");
const after = wasm.memory.buffer.byteLength;
assert.ok(after <= before + 65536 * 4, `memory grew from ${before} to ${after}: a buffer leaks`);

console.log("origami.wasm smoke test passed");
