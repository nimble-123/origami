// HTML for code, shared by the page (in the browser) and site/fill.mjs (in
// Node, at build time), so the page without scripting shows exactly what the
// page with scripting draws. Pure functions: strings in, strings out.

const ESC = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" };
export const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ESC[c]);

/** Tokyo Night syntax colors for ABAP. Groups: 1 comment, 2 string,
 * 3 number, 4 word. */
const ABAP_SYNTAX =
  /(^\*.*|".*)|('(?:[^'\n]|'')*'?|`(?:[^`\n]|``)*`?|\|(?:[^|\\\n]|\\.)*\|?)|\b(\d+)\b|(?<![-~>\w/%$])([A-Za-z_/%$][\w/%$]*(?:-[A-Za-z_][\w]*)*)/gm;

/** `keywords`: the upper-case words origami's ABAP parser knows (from
 * keywords.js). A word is colored as a keyword when it is in the list. */
export function paintHtml(text, keywords) {
  let out = "";
  let pos = 0;
  for (const m of text.matchAll(ABAP_SYNTAX)) {
    let cls = m[1] ? "com" : m[2] ? "str" : m[3] ? "num" : null;
    if (!cls) {
      if (!keywords.has(m[4].toUpperCase())) continue;
      cls = "kw";
    }
    out += esc(text.slice(pos, m.index)) + `<span class="${cls}">${esc(m[0])}</span>`;
    pos = m.index + m[0].length;
  }
  return out + esc(text.slice(pos));
}

/** The lines of a review result as rows: gutter number, code, and the
 * `↳` notes under the construct they explain. `marks`: line numbers (1-based)
 * of lines origami broke, drawn with a valley bar. */
export function codeRows(text, keywords, { notes = [], numbers = true } = {}) {
  const lines = text.replace(/\n$/, "").split("\n");
  const broken = new Set();
  const after = new Map();
  for (const n of notes) {
    for (let l = n.first; l <= n.last; l++) broken.add(l);
    if (!after.has(n.last)) after.set(n.last, []);
    after.get(n.last).push(n);
  }
  let html = "";
  lines.forEach((line, i) => {
    const n = i + 1;
    const cls = broken.has(n) ? "ln add" : "ln";
    html += `<div class="${cls}">${numbers ? `<span class="gut">${n}</span>` : ""}<span class="src">${paintHtml(line, keywords) || " "}</span></div>`;
    for (const note of after.get(n) ?? []) {
      const rule = note.rule ? `<a class="rule" href="https://github.com/nimble-123/origami/blob/main/docs/STYLE.md#:~:text=${encodeURIComponent(note.rule)}">${esc(note.rule)}</a>` : "";
      const rest = note.rule ? note.text.slice(note.rule.length) : note.text;
      html += `<div class="ln note">${numbers ? '<span class="gut"></span>' : ""}<span class="src"><span class="mark">↳ </span>${rule}${esc(rest)}</span></div>`;
    }
  });
  return html;
}

/** The input's lines as rows; a line that does not appear in the output as
 * it is gets a mountain bar: origami changes it. */
export function inputRows(text, output, keywords) {
  const out = new Set(output.replace(/\n$/, "").split("\n"));
  return text
    .replace(/\n$/, "")
    .split("\n")
    .map((line, i) => `<div class="${out.has(line) ? "ln" : "ln del"}"><span class="gut">${i + 1}</span><span class="src">${paintHtml(line, keywords) || " "}</span></div>`)
    .join("");
}

/** What origami changed between input and output, on the output: a mark
 * for each inserted line break, the indentation, each changed run of spaces
 * and each word whose case changed. The two texts have the same tokens. */
export function layoutMarks(input, output, keywords) {
  const words = (t) => {
    const out = [];
    t.replace(/\n$/, "")
      .split("\n")
      .forEach((line, y) => {
        for (const m of line.matchAll(/\S+/g)) out.push({ text: m[0], x: m.index, y, line });
      });
    return out;
  };
  const a = words(input);
  const b = words(output);
  const lines = output.replace(/\n$/, "").split("\n");
  let html = "";
  let k = 0;
  lines.forEach((line, y) => {
    let row = "";
    let pos = 0;
    const indent = line.match(/^ */)[0].length;
    if (indent) row += `<span class="ws indent">${"·".repeat(indent)}</span>`;
    pos = indent;
    while (k < b.length && b[k].y === y) {
      const t = b[k];
      const prev = a[k - 1];
      const now = a[k];
      if (t.x > pos) {
        // Spaces between two tokens of a line: marked when their count changed.
        const gap = t.x - pos;
        const before = now && prev && now.y === prev.y ? now.x - (prev.x + prev.text.length) : -1;
        row += gap !== before ? `<span class="ws">${"·".repeat(gap)}</span>` : " ".repeat(gap);
      }
      const cased = now && now.text !== t.text && now.text.toLowerCase() === t.text.toLowerCase();
      // Only the word is marked, not the period or parenthesis glued to it.
      const word = cased ? t.text.match(/^[A-Za-z_][\w-]*/)?.[0] ?? t.text : "";
      row += cased
        ? `<span class="case">${paintHtml(word, keywords)}</span>${paintHtml(t.text.slice(word.length), keywords)}`
        : paintHtml(t.text, keywords);
      pos = t.x + t.text.length;
      k++;
    }
    // A line break origami added: the next token was on this line before.
    const next = a[k];
    const last = a[k - 1];
    if (next && last && next.y === last.y) row += '<span class="br" title="line break">↵</span>';
    // An empty line origami added (the input has none) is a line break too.
    html += `<div class="ln">${row || '<span class="br" title="blank line">↵</span>'}</div>`;
  });
  return html;
}

/** ANSI SGR colors (as `origami --review` prints them) to spans. */
export function ansiHtml(text) {
  const map = { 1: "b", 2: "dim", 7: "rev", 31: "red", 32: "green", 33: "yellow", 34: "blue", 35: "violet", 36: "cyan" };
  let open = [];
  let html = "";
  const parts = text.split(/\x1b\[([\d;]*)m/);
  parts.forEach((part, i) => {
    if (i % 2 === 0) {
      if (!part) return;
      html += open.length ? `<span class="${open.join(" ")}">${esc(part)}</span>` : esc(part);
      return;
    }
    for (const code of part.split(";").map(Number)) {
      if (code === 0 || Number.isNaN(code)) open = [];
      else if (map[code] && !open.includes(map[code])) open.push(map[code]);
    }
  });
  return html;
}
