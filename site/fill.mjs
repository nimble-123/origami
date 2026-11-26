// Fills the code the website shows without scripting into site/dist: every
// formatted line comes from origami itself, never from hand. site/build.sh
// runs it after building origami.wasm:
//   node site/fill.mjs site/dist
// The inputs live in site/samples/: the sample class and its origami.toml,
// and review.ansi, the output of `origami --review` on it (regenerate that
// with site/samples/review.sh after a formatter change). With scripting, the
// page computes the same views live from the same samples and origami.wasm.
// Markers in index.html: <!-- fill:NAME --> … <!-- /fill -->.

import { readFile, writeFile } from "node:fs/promises";
import { codeRows, inputRows, layoutMarks, ansiHtml, esc, paintHtml } from "./render.js";
import { SAMPLE_WIDTH, CHECK_SOURCE, LAYOUT_SOURCE } from "./samples.js";

const dist = process.argv[2] ?? "site/dist";
const here = new URL(".", import.meta.url);
const { instance } = await WebAssembly.instantiate(await readFile(`${dist}/origami.wasm`), {});
const w = instance.exports;
function call(fn, source, ...args) {
  const bytes = new TextEncoder().encode(source);
  const ptr = w.alloc(bytes.length);
  new Uint8Array(w.memory.buffer, ptr, bytes.length).set(bytes);
  const res = w[fn](ptr, bytes.length, ...args);
  const len = w.result_len();
  const json = new TextDecoder().decode(new Uint8Array(w.memory.buffer, res, len));
  w.dealloc(res, len);
  return JSON.parse(json);
}
const review = (source, width) => call("review", source, 0, width, 2, 0, 0);

const kwSrc = await readFile(`${dist}/keywords.js`, "utf8");
const keywords = new Set(kwSrc.match(/abap: "([^"]*)"/)[1].split(" ").filter(Boolean));

const sample = await readFile(new URL("samples/zcl_flight_check.clas.abap", here), "utf8");
const hero = review(sample, SAMPLE_WIDTH);
const check = review(`${CHECK_SOURCE}\n`, SAMPLE_WIDTH);
const layout = review(`${LAYOUT_SOURCE}\n`, SAMPLE_WIDTH);
for (const [name, r] of [["sample", hero], ["check", check], ["layout", layout]]) {
  if (r.output == null) throw new Error(`fill.mjs: origami did not format the ${name} sample`);
}

// The check: the output's tokens, each underlined, and the count.
const lexIn = call("lex", `${CHECK_SOURCE}\n`, 0);
const lexOut = call("lex", check.output, 0);
const outText = check.output.replace(/\n$/, "");
let checkHtml = "";
let pos = 0;
for (const t of lexOut.tokens) {
  checkHtml += esc(outText.slice(pos, t.start)) + `<u>${esc(outText.slice(t.start, t.end))}</u>`;
  pos = t.end;
}
checkHtml += esc(outText.slice(pos));
const same = lexIn.tokens.length === lexOut.tokens.length;

const review_ = await readFile(new URL("samples/review.ansi", here), "utf8");
const fills = {
  "hero-in": inputRows(sample, hero.output, keywords),
  "hero-out": codeRows(hero.output, keywords, { notes: hero.notes }),
  "check-out": checkHtml,
  "check-count": same ? String(lexOut.tokens.length) : "?",
  "layout-out": layoutMarks(`${LAYOUT_SOURCE}\n`, layout.output, keywords),
  "layout-in": LAYOUT_SOURCE.split("\n").map((l) => paintHtml(l, keywords)).join("\n"),
  "review": ansiHtml(review_),
};

const file = `${dist}/index.html`;
let html = await readFile(file, "utf8");
for (const [name, value] of Object.entries(fills)) {
  const re = new RegExp(`(<!-- fill:${name} -->)[\\s\\S]*?(<!-- /fill -->)`, "g");
  if (!re.test(html)) throw new Error(`fill.mjs: no marker fill:${name} in index.html`);
  html = html.replace(re, (_, a, b) => a + value + b);
}
await writeFile(file, html);
console.log(`fill.mjs: ${Object.keys(fills).length} views filled from origami.wasm`);
