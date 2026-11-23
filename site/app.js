// origami website: the fold animation in the hero, copy buttons and the
// playground (origami compiled to WebAssembly, see crates/origami_wasm).

const REPO = "https://github.com/nimble-123/origami";

for (const a of document.querySelectorAll("[data-repo-path]")) {
  a.href = `${REPO}/blob/main/${a.dataset.repoPath}`;
}

const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

// ---- hero: a long line folds into shape ---------------------------------

const HERO_BEFORE =
  "DATA(ls_rule) = zcl_flight_rules=>get( iv_carrier = is_flight-carrid iv_date = sy-datum iv_mode = 'A' ).";
const HERO_AFTER = [
  "DATA(ls_rule) = zcl_flight_rules=>get( iv_carrier = is_flight-carrid",
  "                                       iv_date    = sy-datum",
  "                                       iv_mode    = 'A' ).",
].join("\n");

/** Tokyo Night syntax colors for ABAP or CDS: comments, strings, numbers
 * and keywords. Groups: 1 comment, 2 string, 3 number, 4 word. */
const ABAP_SYNTAX =
  /(^\*.*|".*)|('(?:[^'\n]|'')*'?|`(?:[^`\n]|``)*`?|\|(?:[^|\\\n]|\\.)*\|?)|\b(\d+)\b|(?<![-~>\w/%$])([A-Za-z_/%$][\w/%$]*(?:-[A-Za-z_][\w]*)*)/gm;
const CDS_SYNTAX =
  /(\/\/.*|--.*|\/\*[\s\S]*?(?:\*\/|$(?![\s\S])))|('(?:[^'\n]|'')*'?)|\b(\d+)\b|(?<![.@#$\w])([A-Za-z_][\w$#]*)/gm;

// Keywords, in upper case: the words origami's parsers know (keywords.js,
// written by site/build.sh), and for CDS the words its grammar matches by
// token kind.
const KEYWORDS = {
  abap: new Set((self.KEYWORDS?.abap ?? "").split(" ").filter(Boolean)),
  cds: new Set(
    [
      ...(self.KEYWORDS?.cds ?? "").split(" "),
      ..."abstract action all annotate annotation aspect between cast child custom default delete determination distinct entity enum exists expose false field from function in is like managed mandatory not null of parent readonly redirected returns role root service strict table then to true type unmanaged update use validation view virtual".split(
        " ",
      ),
    ]
      .filter(Boolean)
      .map((w) => w.toUpperCase()),
  ),
};

/** Text nodes and spans for `text`. `upper`: in ABAP, a word is a keyword
 * when it is in upper case (origami's output with keywords in upper case);
 * otherwise it is a keyword when it is in the keyword list. */
function paint(text, { cds = false, upper = false } = {}) {
  const out = [];
  let pos = 0;
  const words = cds ? KEYWORDS.cds : KEYWORDS.abap;
  for (const m of text.matchAll(cds ? CDS_SYNTAX : ABAP_SYNTAX)) {
    let cls = m[1] ? "com" : m[2] ? "str" : m[3] ? "num-lit" : null;
    if (!cls) {
      const w = m[4];
      const kw =
        upper && !cds ? /^[A-Z][A-Z-]*[A-Z]$/.test(w) : words.has(w.toUpperCase());
      if (!kw) continue;
      cls = "kw";
    }
    if (m.index > pos) out.push(text.slice(pos, m.index));
    const s = document.createElement("span");
    s.className = cls;
    s.textContent = m[0];
    out.push(s);
    pos = m.index + m[0].length;
  }
  if (pos < text.length) out.push(text.slice(pos));
  return out;
}

/** Tokens with line and column. origami changes layout only, so the two
 * texts have the same tokens in the same order. */
function tokens(text) {
  const out = [];
  text.split("\n").forEach((line, y) => {
    for (const m of line.matchAll(/\S+/g)) out.push({ text: m[0], x: m.index, y });
  });
  return out;
}

function hero() {
  const box = document.getElementById("hero-code");
  const note = document.getElementById("hero-note");
  if (!box) return;
  const before = tokens(HERO_BEFORE);
  const after = tokens(HERO_AFTER);
  const spans = before.map((t, i) => {
    const s = document.createElement("span");
    s.className = "tok";
    s.append(...paint(t.text, { upper: true }));
    s.style.setProperty("--x", t.x);
    s.style.setProperty("--y", t.y);
    box.append(s);
    return s;
  });
  // A crease before every token that starts a new line after the fold.
  let k = 0;
  after.forEach((t, i) => {
    if (t.y > 0 && (i === 0 || after[i - 1].y !== t.y)) {
      const c = document.createElement("span");
      c.className = "crease";
      c.style.setProperty("--x", before[i].x);
      c.style.setProperty("--i", k++);
      box.append(c);
    }
  });
  const fold = () => {
    after.forEach((t, i) => {
      spans[i].style.setProperty("--x", t.x);
      spans[i].style.setProperty("--y", t.y);
    });
    box.classList.add("folded");
  };
  if (reduceMotion) {
    for (const s of spans) s.style.transition = "none";
    fold();
    note.classList.add("show");
    return;
  }
  setTimeout(() => box.classList.add("creased"), 500);
  setTimeout(fold, 1300);
  setTimeout(() => note.classList.add("show"), 2100);
}

// ---- copy buttons ---------------------------------------------------------

for (const b of document.querySelectorAll("[data-copy]")) {
  b.addEventListener("click", async () => {
    const src = document.querySelector(b.dataset.copy);
    try {
      await navigator.clipboard.writeText(src.textContent);
    } catch {
      return;
    }
    b.classList.add("done");
    if (b.classList.contains("text")) b.textContent = "Copied";
    setTimeout(() => {
      b.classList.remove("done");
      if (b.classList.contains("text")) b.textContent = "Copy";
    }, 1600);
  });
}

// ---- playground -------------------------------------------------------------

const EXAMPLES = {
  call: {
    lang: "abap",
    text: `CLASS zcl_flight_check DEFINITION PUBLIC FINAL CREATE PUBLIC.
  PUBLIC SECTION.
    METHODS check IMPORTING is_flight TYPE sflight iv_mode TYPE c DEFAULT 'A' iv_strict TYPE abap_bool DEFAULT abap_false RETURNING VALUE(rv_ok) TYPE abap_bool.
ENDCLASS.

CLASS zcl_flight_check IMPLEMENTATION.
  METHOD check.
    DATA(ls_rule) = zcl_flight_rules=>get( iv_carrier = is_flight-carrid iv_connection = is_flight-connid iv_date = sy-datum iv_mode = iv_mode ).
    data lv_count type i. lv_count = lines( ls_rule-exceptions ).
    if ls_rule-active = abap_true and lv_count = 0 and is_flight-seatsmax > is_flight-seatsocc and ( iv_mode <> 'X' or iv_strict = abap_false ).
      rv_ok = abap_true.
    endif.
  ENDMETHOD.
ENDCLASS.
`,
  },
  sql: {
    lang: "abap",
    text: `REPORT zflights.
SELECT carrid, connid, fldate, price, currency FROM sflight WHERE carrid = @p_carr AND fldate >= @sy-datum ORDER BY fldate INTO TABLE @DATA(lt_flights).
UPDATE zflight_stats SET last_run = @sy-datum, flights = @( lines( lt_flights ) ) WHERE carrid = @p_carr AND connid = @p_conn.
LOOP AT lt_flights INTO DATA(ls_flight) WHERE price > 1000. WRITE: / ls_flight-carrid, ls_flight-connid, ls_flight-price. ENDLOOP.
`,
  },
  cds: {
    lang: "cds",
    text: `@AbapCatalog.viewEnhancementCategory: [#NONE] @AccessControl.authorizationCheck: #CHECK
@EndUserText.label: 'Flights with price class'
define view entity ZI_FlightPrice as select from sflight association [0..1] to ZI_Carrier as _Carrier on $projection.carrid = _Carrier.carrid {
key carrid, key connid, key fldate,
price, currency,
case when price > 1000 then 'expensive' when price > 500 then 'medium' else 'cheap' end as price_class,
_Carrier
}
`,
  },
};

const $ = (id) => document.getElementById(id);
const input = $("pg-input");
const highlight = $("pg-highlight");
const output = $("pg-output");
const outputText = $("pg-output-text");
const status = $("pg-status");

// origami runs in a Web Worker (worker.js). A run that takes longer than
// RUN_TIMEOUT_MS terminates the worker and starts a new one.
const RUN_TIMEOUT_MS = 5000;
const LOAD_ERROR = "The playground could not load origami.wasm. Install origami to try it locally.";

let worker = null;
let ready = false;
let nextId = 0;
let inflight = null; // { id, timer }
let queued = null; // the newest request made while a run was in flight

function ruleLink(text, rule) {
  const span = document.createElement("span");
  if (!rule) {
    span.textContent = text;
    return span;
  }
  const a = document.createElement("a");
  a.href = `${REPO}/blob/main/docs/STYLE.md#:~:text=${encodeURIComponent(rule)}`;
  a.textContent = rule;
  a.target = "_blank";
  a.rel = "noopener";
  span.append(a, text.slice(rule.length));
  return span;
}

function render(r) {
  output.replaceChildren();
  if (r.status === "error" || r.output == null) {
    const d = r.diagnostics.find((x) => x.severity !== "warning") ?? r.diagnostics[0];
    const p = document.createElement("p");
    p.className = "empty";
    p.textContent = "Nothing formatted: the code has a structural error, so origami leaves it as it is.";
    output.append(p);
    outputText.textContent = "";
    status.classList.add("error");
    status.textContent = d
      ? `${d.code} at line ${d.line}, column ${d.column}: ${d.message}`
      : "origami could not format this code.";
    return;
  }
  status.classList.remove("error");
  outputText.textContent = r.output;
  const lines = r.output.replace(/\n$/, "").split("\n");
  const broken = new Set();
  const after = new Map();
  for (const n of r.notes) {
    for (let l = n.first; l <= n.last; l++) broken.add(l);
    if (!after.has(n.last)) after.set(n.last, []);
    after.get(n.last).push(n);
  }
  const frag = document.createDocumentFragment();
  lines.forEach((text, i) => {
    const n = i + 1;
    const row = document.createElement("div");
    row.className = broken.has(n) ? "ln broken" : "ln";
    row.dataset.n = n;
    const code = document.createElement("span");
    code.append(...(text ? paint(text, { cds: options().cds, upper: !options().lower }) : [" "]));
    row.append(code);
    frag.append(row);
    for (const note of after.get(n) ?? []) {
      const p = document.createElement("p");
      p.className = "note ln-note";
      const mark = document.createElement("span");
      mark.className = "note-mark";
      mark.setAttribute("aria-hidden", "true");
      mark.textContent = "↳ ";
      p.append(mark, ruleLink(note.text, note.rule));
      frag.append(p);
    }
  });
  output.append(frag);
  const warnings = r.diagnostics.filter((d) => d.code !== "W003").length;
  const num = (n) => `<span class="num">${n}</span>`;
  const parts = [];
  parts.push(r.changed ? "Formatted" : "Already formatted");
  parts.push(`${num(r.notes.length)} ${r.notes.length === 1 ? "note" : "notes"} on line breaks`);
  if (warnings) parts.push(`${num(warnings)} ${warnings === 1 ? "warning" : "warnings"}`);
  status.innerHTML = `${parts.join(", ")}. Took ${num(r.ms)} ms in your browser.`;
}

// Inputs larger than this are shown without colors, so typing stays fast.
const HIGHLIGHT_MAX = 200_000;

/** Colors the copy of the input behind the textarea. */
function paintInput() {
  const text = input.value;
  // The trailing line keeps the last line reachable when the textarea scrolls.
  highlight.replaceChildren(
    ...(text.length > HIGHLIGHT_MAX ? [text] : paint(text, { cds: options().cds })),
    "\n",
  );
  syncScroll();
}

function syncScroll() {
  highlight.scrollTop = input.scrollTop;
  highlight.scrollLeft = input.scrollLeft;
}

function options() {
  return {
    cds: $("pg-lang").value === "cds",
    width: Number($("pg-width").value),
    lower: $("pg-case").value === "lower",
  };
}

let timer = 0;
function schedule() {
  clearTimeout(timer);
  timer = setTimeout(run, 120);
}

function showError(message) {
  status.classList.add("error");
  status.textContent = message;
}

function startWorker() {
  ready = false;
  const mine = new Worker("worker.js");
  worker = mine;
  mine.onmessage = ({ data }) => {
    if (mine !== worker) return;
    if (data.type === "ready") {
      ready = true;
      if (queued) dispatch();
      else run();
    } else if (data.type === "load-error") {
      showError(LOAD_ERROR);
    } else if (inflight && data.id === inflight.id) {
      clearTimeout(inflight.timer);
      inflight = null;
      if (data.type === "panic") {
        showError("origami stopped on this input (internal error). Please report it with the code.");
      } else {
        render(data.result);
      }
      if (queued) dispatch();
    }
  };
  mine.onerror = () => showError(LOAD_ERROR);
}

function dispatch() {
  const job = queued;
  queued = null;
  const id = nextId++;
  const timer = setTimeout(() => {
    // The worker is stuck in a long run: stop it and start a new one.
    worker.terminate();
    inflight = null;
    showError(
      `origami took longer than ${RUN_TIMEOUT_MS / 1000} seconds on this input and was stopped. Try a smaller input.`,
    );
    startWorker();
  }, RUN_TIMEOUT_MS);
  inflight = { id, timer };
  worker.postMessage({ id, ...job });
}

function run() {
  paintInput();
  if (!ready) return;
  queued = { source: input.value, opts: options() };
  if (!inflight) dispatch();
}

function loadExample(key) {
  const ex = EXAMPLES[key];
  input.value = ex.text;
  $("pg-lang").value = ex.lang;
  run();
}

async function playground() {
  if (!input) return;
  input.value = EXAMPLES.call.text;
  $("pg-example").addEventListener("change", (e) => loadExample(e.target.value));
  for (const id of ["pg-lang", "pg-width", "pg-case"]) $(id).addEventListener("change", run);
  input.addEventListener("input", () => {
    paintInput();
    schedule();
  });
  input.addEventListener("scroll", syncScroll);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Tab" && !e.shiftKey && !e.altKey && !e.metaKey && !e.ctrlKey) {
      e.preventDefault();
      input.setRangeText("  ", input.selectionStart, input.selectionEnd, "end");
      paintInput();
      schedule();
    }
  });
  try {
    startWorker();
  } catch {
    showError(LOAD_ERROR);
  }
}

hero();
playground();
