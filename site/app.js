// origami website: the hero (a long line folds into shape, and a margin that
// can be dragged reformats it with the real formatter), the 3D crane next to
// it (crane.js, loaded later), copy buttons, the playground (origami compiled
// to WebAssembly, see crates/origami_wasm) and the install tabs. One worker
// (worker.js) formats for the hero and the playground.

const REPO = "https://github.com/nimble-123/origami";

for (const a of document.querySelectorAll("[data-repo-path]")) {
  a.href = `${REPO}/blob/main/${a.dataset.repoPath}`;
}

const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
const $ = (id) => document.getElementById(id);
const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

// ---- syntax colors ----------------------------------------------------------

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

/** Tokens with line and column. origami changes layout only, so two
 * layouts of the same code have the same tokens in the same order. */
function tokens(text) {
  const out = [];
  text.split("\n").forEach((line, y) => {
    for (const m of line.matchAll(/\S+/g)) out.push({ text: m[0], x: m.index, y });
  });
  return out;
}

// ---- announcements ----------------------------------------------------------

// A visually hidden live region: the page says what changed, but only for
// state changes the visitor asked for (a copy, an error, a finished run).
const announcer = $("announce");
let announceTimer = 0;
function announce(message, delay = 0) {
  clearTimeout(announceTimer);
  // Emptied first, so the same message twice is announced twice.
  if (announcer) announcer.textContent = "";
  announceTimer = setTimeout(() => {
    if (announcer) announcer.textContent = message;
  }, delay);
}

// ---- the formatter: one worker for the hero and the playground ------------

// origami runs in a Web Worker (worker.js). A run that takes longer than
// RUN_TIMEOUT_MS terminates the worker and starts a new one.
const RUN_TIMEOUT_MS = 5000;

/** Requests by channel ("hero", "pg"): only the newest request of a channel
 * waits; the hero goes first, so dragging stays direct while the playground
 * formats a large file. */
const formatter = {
  worker: null,
  ready: false,
  failed: false,
  nextId: 0,
  inflight: null, // { id, timer, job, channel }
  queue: new Map(),
  readyHandlers: [],
  failHandlers: [],
};

function startWorker() {
  formatter.ready = false;
  const mine = new Worker("worker.js");
  formatter.worker = mine;
  mine.onmessage = ({ data }) => {
    if (mine !== formatter.worker) return;
    if (data.type === "ready") {
      formatter.ready = true;
      for (const f of formatter.readyHandlers) f();
      dispatch();
    } else if (data.type === "load-error") {
      formatter.failed = true;
      for (const f of formatter.failHandlers) f();
    } else if (formatter.inflight && data.id === formatter.inflight.id) {
      const { timer, job, channel } = formatter.inflight;
      clearTimeout(timer);
      formatter.inflight = null;
      // A result that a newer request of its channel replaces is dropped.
      if (!(job.latestOnly && formatter.queue.has(channel))) job.onResult(data);
      dispatch();
    }
  };
  mine.onerror = () => {
    formatter.failed = true;
    for (const f of formatter.failHandlers) f();
  };
}

function submit(channel, job) {
  formatter.queue.set(channel, job);
  if (formatter.ready && !formatter.inflight) dispatch();
}

function dispatch() {
  if (!formatter.ready || formatter.inflight || !formatter.queue.size) return;
  const channel = formatter.queue.has("hero") ? "hero" : formatter.queue.keys().next().value;
  const job = formatter.queue.get(channel);
  formatter.queue.delete(channel);
  const id = formatter.nextId++;
  const timer = setTimeout(() => {
    // The worker is stuck in a long run: stop it and start a new one.
    formatter.worker.terminate();
    formatter.inflight = null;
    job.onTimeout?.();
    startWorker();
  }, RUN_TIMEOUT_MS);
  formatter.inflight = { id, timer, job, channel };
  formatter.worker.postMessage({ id, source: job.source, opts: job.opts });
}

// ---- hero: a long line folds into shape ---------------------------------

const HERO_SOURCE =
  "DATA(ls_rule) = zcl_flight_rules=>get( iv_carrier = is_flight-carrid iv_date = sy-datum iv_mode = 'A' ).";
const HERO_MIN = 40;
const HERO_MAX = 120;
// origami's output for the call in style "compact" (the layout that keeps a
// call on one line while it fits), shown before origami.wasm is ready: at
// widths 68 to 103, and below 68.
const HERO_WIDE = [
  "DATA(ls_rule) = zcl_flight_rules=>get( iv_carrier = is_flight-carrid",
  "                                       iv_date    = sy-datum",
  "                                       iv_mode    = 'A' ).",
].join("\n");
const HERO_NARROW = [
  "DATA(ls_rule) = zcl_flight_rules=>get(",
  "        iv_carrier = is_flight-carrid",
  "        iv_date    = sy-datum",
  "        iv_mode    = 'A' ).",
].join("\n");
const HERO_OPTS = { cds: false, lower: false, compact: true };
// The load moment, in ms from the start: creases, fold, note, nudge.
const MOMENT = { crease: 450, fold: 1000, foldFor: 640, note: 1600, nudge: 1900 };

function heroStaticResult(w) {
  if (w > HERO_SOURCE.length) return { output: HERO_SOURCE, notes: [] };
  return {
    output: w >= 68 ? HERO_WIDE : HERO_NARROW,
    notes: [{ text: `ABAP-CALL-001: one parameter per line, because the call would end at column 103 (limit ${w})` }],
  };
}

/** How far the crane is folded for a layout: one line is almost flat, more
 * lines fold it further, and within a layout a narrower margin a little more. */
function foldFor(w, lines) {
  const base = lines <= 1 ? 0.05 : lines <= 3 ? 0.62 : 0.84;
  return clamp(base + 0.18 * ((HERO_MAX - w) / (HERO_MAX - HERO_MIN)), 0, 1);
}

const hero = {
  w: 72,
  spans: [],
  lines: 1,
  cache: new Map(),
  raf: 0,
  active: false,
  folded: false, // the load moment has folded the line
  momentDone: false,
  crane: null,
  t0: performance.now(),
};

function heroLayout(text, { dur = 220, stagger = 0, instant = false } = {}) {
  const box = $("hero-code");
  const toks = tokens(text);
  const same = toks.length === hero.spans.length && toks.every((t, i) => t.text === hero.spans[i].dataset.t);
  box.classList.toggle("instant", instant || reduceMotion || !same);
  box.style.setProperty("--dur", `${dur}ms`);
  box.style.setProperty("--stagger", `${stagger}ms`);
  if (!same) {
    for (const s of hero.spans) s.remove();
    hero.spans = toks.map((t) => {
      const s = document.createElement("span");
      s.className = "tok";
      s.dataset.t = t.text;
      s.append(...paint(t.text, { upper: true }));
      box.append(s);
      return s;
    });
  }
  toks.forEach((t, i) => {
    hero.spans[i].style.setProperty("--x", t.x);
    hero.spans[i].style.setProperty("--y", t.y);
  });
  const lines = text.split("\n");
  hero.lines = lines.length;
  box.style.setProperty("--cols", Math.max(...lines.map((l) => l.length)));
  box.style.setProperty("--lines", Math.max(3, lines.length));
  if (!same && !instant && !reduceMotion) {
    // New spans start where they are; later changes move them.
    box.getBoundingClientRect();
    box.classList.remove("instant");
  }
}

function heroNote(result, w) {
  const text = result.output.replace(/\n$/, "");
  const n = result.notes.find((x) => x.text.includes("because")) ?? result.notes[0];
  const longest = Math.max(...text.split("\n").map((l) => l.length));
  $("hero-note-text").textContent = n
    ? n.text
    : `No line break: the statement ends at column ${longest} (limit ${w})`;
  $("hero-static").textContent = text;
  $("hero-caption").textContent = `A method call that ends at column 103, formatted at a line width of ${w}${
    n ? ", with the note that explains the break" : ": it fits on one line"
  }.`;
}

function heroShow(result, w, opts) {
  if (result.status === "error" || result.output == null) return;
  heroLayout(result.output.replace(/\n$/, ""), opts);
  heroNote(result, w);
  hero.crane?.setFold(foldFor(w, hero.lines));
}

function heroFormat() {
  if (hero.raf) return;
  hero.raf = requestAnimationFrame(() => {
    hero.raf = 0;
    const w = hero.w;
    const cached = hero.cache.get(w);
    if (cached) return hero.folded && heroShow(cached, w);
    if (!formatter.ready) return;
    submit("hero", {
      source: `${HERO_SOURCE}\n`,
      opts: { ...HERO_OPTS, width: w },
      latestOnly: true,
      onResult: (data) => {
        if (data.type !== "result") return;
        hero.cache.set(w, data.result);
        // A result for a width the margin has already left is not shown,
        // and none before the load moment has folded the line.
        if (hero.w === w && hero.folded) heroShow(data.result, w);
      },
    });
  });
}

/** The width of one column in the hero, in CSS pixels. */
function heroCh() {
  const canvas = $("hero-canvas");
  // Until Martian Mono has loaded, its advance: 0.7em.
  if (!document.fonts.check(`12px "Martian Mono"`)) return parseFloat(getComputedStyle(canvas).fontSize) * 0.7;
  const probe = document.createElement("span");
  probe.textContent = "0".repeat(20);
  probe.style.position = "absolute";
  probe.style.visibility = "hidden";
  canvas.append(probe);
  const ch = probe.getBoundingClientRect().width / 20;
  probe.remove();
  return ch || 8;
}

/** The widest margin that fits the visible part of the sheet. */
function heroFit() {
  const scroll = $("hero-scroll");
  const pad = parseFloat(getComputedStyle(scroll).paddingLeft) || 0;
  return Math.floor((scroll.clientWidth - 2 * pad - 26) / heroCh());
}

function setHeroWidth(w) {
  w = clamp(Math.round(w), HERO_MIN, HERO_MAX);
  if (w === hero.w) return;
  hero.w = w;
  const margin = $("hero-margin");
  $("hero-canvas").style.setProperty("--w", w);
  $("hero-width").textContent = w;
  margin.setAttribute("aria-valuenow", w);
  margin.setAttribute("aria-valuetext", `line width ${w}`);
  heroFormat();
}

/** Keeps the margin inside the visible part of the sheet. */
function heroReveal() {
  const scroll = $("hero-scroll");
  const m = $("hero-margin").getBoundingClientRect();
  const s = scroll.getBoundingClientRect();
  if (m.right > s.right) scroll.scrollLeft += m.right - s.right + 8;
  else if (m.left < s.left) scroll.scrollLeft -= s.left - m.left + 8;
}

function heroActivate() {
  if (hero.active) return;
  hero.active = true;
  const margin = $("hero-margin");
  margin.removeAttribute("aria-disabled");
  $("hero-hint").classList.add("show");
  if (hero.momentDone && !reduceMotion) heroNudge();
}

function heroNudge() {
  const margin = $("hero-margin");
  margin.classList.add("nudge");
  margin.addEventListener("animationend", () => margin.classList.remove("nudge"), { once: true });
}

function heroMargin() {
  const margin = $("hero-margin");
  const scroll = $("hero-scroll");
  const canvas = $("hero-canvas");
  margin.addEventListener("keydown", (e) => {
    if (!hero.active) return;
    const step = { ArrowRight: 1, ArrowUp: 1, ArrowLeft: -1, ArrowDown: -1, PageUp: 10, PageDown: -10 }[e.key];
    let w = null;
    if (step) w = hero.w + step;
    else if (e.key === "Home") w = HERO_MIN;
    else if (e.key === "End") w = HERO_MAX;
    if (w == null) return;
    e.preventDefault();
    setHeroWidth(w);
    heroReveal();
  });

  // Dragging: the pointer sets the column; near the edge of the sheet the
  // code scrolls, so the margin can reach 120 on a small screen too.
  let drag = null;
  const fromPointer = () => {
    const left = canvas.getBoundingClientRect().left;
    setHeroWidth((drag.x - left) / drag.ch);
  };
  const edge = () => {
    if (!drag) return;
    const s = scroll.getBoundingClientRect();
    const zone = 36;
    let dx = 0;
    if (drag.x > s.right - zone) dx = Math.min(1, (drag.x - (s.right - zone)) / zone) * 6;
    else if (drag.x < s.left + zone) dx = -Math.min(1, (s.left + zone - drag.x) / zone) * 6;
    if (dx) {
      scroll.scrollLeft += dx;
      fromPointer();
    }
    drag.edge = requestAnimationFrame(edge);
  };
  margin.addEventListener("pointerdown", (e) => {
    if (!hero.active || (e.pointerType === "mouse" && e.button !== 0)) return;
    margin.setPointerCapture(e.pointerId);
    margin.classList.add("dragging");
    drag = { id: e.pointerId, x: e.clientX, ch: heroCh(), edge: 0 };
    drag.edge = requestAnimationFrame(edge);
  });
  margin.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    drag.x = e.clientX;
    fromPointer();
  });
  const end = (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    cancelAnimationFrame(drag.edge);
    drag = null;
    margin.classList.remove("dragging");
  };
  margin.addEventListener("pointerup", end);
  margin.addEventListener("pointercancel", end);
  margin.addEventListener("lostpointercapture", end);
}

function heroStart() {
  const box = $("hero-code");
  if (!box) return;
  // On a small screen the margin starts where it fits.
  const w = clamp(Math.min(72, heroFit()), HERO_MIN, HERO_MAX);
  hero.w = 0;
  setHeroWidth(w);
  heroMargin();
  const target = heroStaticResult(w);
  if (reduceMotion) {
    hero.folded = true;
    heroShow(target, w, { instant: true });
    $("hero-note").classList.add("show");
    hero.momentDone = true;
    return;
  }
  // The long line, then a crease before every token that starts a new line
  // after the fold, then the fold, the note and a nudge of the margin.
  heroLayout(HERO_SOURCE, { instant: true });
  heroNote(target, w);
  const after = tokens(target.output);
  const before = tokens(HERO_SOURCE);
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
  setTimeout(() => box.classList.add("creased"), MOMENT.crease);
  setTimeout(() => {
    // The margin may have moved (or origami formatted) in the meantime.
    hero.folded = true;
    const now = hero.cache.get(hero.w) ?? heroStaticResult(hero.w);
    heroShow(now, hero.w, { dur: MOMENT.foldFor, stagger: 70 });
    box.classList.add("folded");
  }, MOMENT.fold);
  setTimeout(() => $("hero-note").classList.add("show"), MOMENT.note);
  setTimeout(() => {
    hero.momentDone = true;
    if (hero.active) heroNudge();
  }, MOMENT.nudge);
}

// ---- hero: the 3D crane -------------------------------------------------------

function craneStill(box) {
  box.classList.add("still");
}

function craneLoad() {
  const box = $("hero-crane");
  if (!box) return;
  const conn = navigator.connection;
  if (conn?.saveData || (navigator.deviceMemory && navigator.deviceMemory < 2)) return craneStill(box);
  const probe = document.createElement("canvas");
  // three.js r170 needs WebGL 2.
  const gl = probe.getContext("webgl2");
  if (!gl) return craneStill(box);
  gl.getExtension("WEBGL_lose_context")?.loseContext();
  const go = () =>
    import("./crane.js")
      .then((m) => {
        hero.crane = m.mount(box, {
          target: foldFor(hero.w, (hero.cache.get(hero.w) ?? heroStaticResult(hero.w)).output.trim().split("\n").length),
          moment: reduceMotion ? null : { t0: hero.t0, ...MOMENT },
          reduceMotion,
          onFail: () => {
            hero.crane = null;
            craneStill(box);
          },
        });
      })
      .catch(() => craneStill(box));
  // After the first paint, when the main thread is idle.
  requestAnimationFrame(() => setTimeout(() => ("requestIdleCallback" in window ? requestIdleCallback(go, { timeout: 400 }) : go()), 0));
}

// ---- copy buttons ---------------------------------------------------------

for (const b of document.querySelectorAll("[data-copy]")) {
  const label = b.getAttribute("aria-label");
  let reset = 0;
  b.addEventListener("click", async () => {
    const src = document.querySelector(b.dataset.copy);
    const isText = b.classList.contains("text");
    let ok = true;
    try {
      await navigator.clipboard.writeText(src.textContent);
    } catch {
      ok = false;
    }
    clearTimeout(reset);
    b.classList.toggle("done", ok);
    b.classList.toggle("failed", !ok);
    if (isText) {
      b.textContent = ok ? "Copied" : "Copy failed";
      b.setAttribute("aria-label", ok ? "Copied" : "Copy failed");
    }
    announce(ok ? "Copied to the clipboard." : "Copying failed. Select the text and copy it by hand.");
    reset = setTimeout(() => {
      b.classList.remove("done", "failed");
      if (isText) {
        b.textContent = "Copy";
        b.setAttribute("aria-label", label);
      }
    }, 1600);
  });
}

// ---- tabs (playground panes, install) -----------------------------------------

/** Arrow keys, Home and End move between the tabs and select them. */
function tabKeys(tabs, select) {
  tabs.forEach((tab, i) => {
    tab.addEventListener("click", () => select(i));
    tab.addEventListener("keydown", (e) => {
      const n = tabs.length;
      const j = { ArrowRight: (i + 1) % n, ArrowLeft: (i - 1 + n) % n, Home: 0, End: n - 1 }[e.key];
      if (j == null) return;
      e.preventDefault();
      select(j);
      tabs[j].focus();
    });
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

const input = $("pg-input");
const highlight = $("pg-highlight");
const output = $("pg-output");
const outputText = $("pg-output-text");
const status = $("pg-status");
const copyButton = $("pg-copy");

// A finished run is announced to screen readers only after this pause, so
// typing is not interrupted by a message per keystroke.
const ANNOUNCE_PAUSE_MS = 1500;
const LOAD_ERROR = "The playground could not load origami.wasm. Install origami to try it locally.";
// Files larger than this are not opened: the playground is for trying origami.
const FILE_MAX = 5_000_000;

let timedOut = null; // the request that hit the timeout; not run again until it changes

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

/** Empties the output, so a stale result is neither visible nor copyable. */
function clearOutput(message) {
  output.replaceChildren();
  output.classList.remove("plain");
  outputText.textContent = "";
  copyButton.disabled = true;
  const p = document.createElement("p");
  p.className = "empty";
  p.textContent = message;
  output.append(p);
}

function formatMs(ms) {
  return ms < 0.1 ? "under 0.1" : ms < 10 ? ms.toFixed(1) : String(Math.round(ms));
}

/** `opts`: the options the result was made with, not the current ones. */
function render(r, opts) {
  output.replaceChildren();
  output.classList.remove("plain");
  if (r.status === "error" || r.output == null) {
    const d = r.diagnostics.find((x) => x.severity !== "warning") ?? r.diagnostics[0];
    clearOutput("Nothing formatted: the code has a structural error, so origami leaves it as it is.");
    status.classList.add("error");
    status.textContent = d
      ? `${d.code} at line ${d.line}, column ${d.column}: ${d.message}`
      : "origami could not format this code.";
    announce(status.textContent);
    panes.changed();
    return;
  }
  status.classList.remove("error");
  outputText.textContent = r.output;
  copyButton.disabled = false;
  const warnings = r.diagnostics.filter((d) => d.code !== "W003").length;
  const num = (n) => `<span class="num">${n}</span>`;
  const parts = [`${r.changed ? "Formatted" : "Already formatted, checked"} in ${num(formatMs(r.ms))} ms`];
  parts.push(`${num(r.notes.length)} line ${r.notes.length === 1 ? "break" : "breaks"} with a note`);
  if (warnings) parts.push(`${num(warnings)} ${warnings === 1 ? "warning" : "warnings"}`);
  // A row and highlight spans per line would freeze the tab on a huge paste.
  if (r.output.length > HIGHLIGHT_MAX) {
    output.classList.add("plain");
    output.textContent = r.output;
    status.innerHTML = `${parts.join(", ")}. Shown as plain text because the output is large.`;
    announce(status.textContent, ANNOUNCE_PAUSE_MS);
    panes.changed();
    return;
  }
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
    code.append(...(text ? paint(text, { cds: opts.cds, upper: !opts.lower }) : [" "]));
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
  status.innerHTML = `${parts.join(", ")}.`;
  announce(status.textContent, ANNOUNCE_PAUSE_MS);
  panes.changed();
}

// Inputs and outputs larger than this are shown without colors, so typing
// stays fast and a huge paste cannot freeze the tab.
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

const radio = (name) => document.querySelector(`input[name="${name}"]:checked`)?.value;
function setRadio(name, value) {
  const r = document.querySelector(`input[name="${name}"][value="${value}"]`);
  if (r) r.checked = true;
}

function options() {
  return {
    cds: radio("pg-lang") === "cds",
    width: Number(radio("pg-width")),
    lower: radio("pg-case") === "lower",
    compact: radio("pg-style") === "compact",
  };
}

let timer = 0;
function schedule() {
  clearTimeout(timer);
  clearTimeout(announceTimer);
  timer = setTimeout(run, 120);
}

function showError(message) {
  clearOutput("Nothing to show: origami did not format this code.");
  status.classList.add("error");
  status.textContent = message;
  announce(message);
}

function run() {
  paintInput();
  if (!formatter.ready) return;
  const job = { source: input.value, opts: options() };
  // The restarted worker would hang on the same request again.
  if (timedOut && timedOut.source === job.source && JSON.stringify(timedOut.opts) === JSON.stringify(job.opts)) return;
  submit("pg", {
    ...job,
    latestOnly: true,
    onResult: (data) => {
      if (data.type === "panic") {
        showError("origami stopped on this input (internal error). Please report it with the code.");
      } else {
        render(data.result, job.opts);
      }
    },
    onTimeout: () => {
      timedOut = job;
      showError(
        `origami took longer than ${RUN_TIMEOUT_MS / 1000} seconds on this input and was stopped. Try a smaller input.`,
      );
    },
  });
}

function loadExample(key) {
  const ex = EXAMPLES[key];
  input.value = ex.text;
  setRadio("pg-lang", ex.lang);
  run();
}

/** The language a file name says: ABAP for `.abap`, the CDS family for the
 * CDS, DDLX, DCL, BDEF and SRVD extensions; `null` when it says nothing. */
function langOf(name) {
  const n = name.toLowerCase();
  if (n.endsWith(".abap")) return "abap";
  if ([".asddls", ".asddlxs", ".asdcls", ".asbdef", ".srvdsrv", ".acds"].some((e) => n.endsWith(e))) return "cds";
  return null;
}

/** Reads a file in the browser; nothing is uploaded. */
async function openFile(file) {
  if (!file) return;
  if (file.size > FILE_MAX) {
    showError(`${file.name} is larger than 5 MB. Open a smaller file, or run origami on it locally.`);
    return;
  }
  let text;
  try {
    text = await file.text();
  } catch {
    showError(`${file.name} could not be read. Try again, or paste its contents.`);
    return;
  }
  input.value = text;
  const lang = langOf(file.name);
  if (lang) setRadio("pg-lang", lang);
  panes.select(0);
  announce(`Opened ${file.name}.`);
  run();
}

/** Below 56rem the panes are tabs: "Your code" and "Formatted". The tab of
 * the output gets a dot when a result arrives while the input is shown. */
const panes = {
  narrow: matchMedia("(max-width: 55.99rem)"),
  index: 0,
  seen: null,
  tabs: [],
  setup() {
    const list = $("pg-tabs");
    this.tabs = [$("pg-tab-in"), $("pg-tab-out")];
    this.panels = [$("pg-pane-in"), $("pg-pane-out")];
    tabKeys(this.tabs, (i) => this.select(i));
    const apply = () => {
      const on = this.narrow.matches;
      list.hidden = !on;
      $("pg").classList.toggle("tabbed", on);
      this.panels.forEach((p, i) => {
        if (on) {
          p.setAttribute("role", "tabpanel");
          p.setAttribute("aria-labelledby", this.tabs[i].id);
          p.hidden = i !== this.index;
        } else {
          p.removeAttribute("role");
          p.removeAttribute("aria-labelledby");
          p.hidden = false;
        }
      });
    };
    this.narrow.addEventListener("change", apply);
    apply();
  },
  select(i) {
    this.index = i;
    this.tabs.forEach((t, j) => {
      t.setAttribute("aria-selected", String(i === j));
      t.tabIndex = i === j ? 0 : -1;
    });
    if (this.narrow.matches) this.panels.forEach((p, j) => (p.hidden = i !== j));
    if (i === 1) {
      this.seen = outputText.textContent;
      this.badge(false);
    }
  },
  changed() {
    const now = outputText.textContent;
    // The first result (the example) is not news.
    if (this.seen === null) this.seen = now;
    if (this.narrow.matches && this.index === 0 && now !== this.seen) this.badge(true);
    if (this.index === 1 || !this.narrow.matches) this.seen = now;
  },
  badge(on) {
    $("pg-badge").classList.toggle("on", on);
    $("pg-badge-text").hidden = !on;
  },
};

function playground() {
  if (!input) return;
  input.value = EXAMPLES.call.text;
  panes.setup();
  $("pg-example").addEventListener("change", (e) => loadExample(e.target.value));
  for (const r of document.querySelectorAll(".pg-controls input[type=radio]")) r.addEventListener("change", run);
  // Tab indents, which would trap keyboard users: Escape releases the trap
  // until the next input or focus.
  let tabIndents = true;
  input.addEventListener("input", () => {
    tabIndents = true;
    paintInput();
    schedule();
  });
  input.addEventListener("focus", () => {
    tabIndents = true;
  });
  input.addEventListener("scroll", syncScroll);
  input.addEventListener("keydown", (e) => {
    if (e.altKey || e.metaKey || e.ctrlKey) return;
    if (e.key === "Escape") {
      tabIndents = false;
    } else if (e.key === "Tab" && !e.shiftKey && tabIndents) {
      e.preventDefault();
      // insertText keeps the browser's undo history; it fires "input" itself.
      if (!document.execCommand("insertText", false, "  ")) {
        input.setRangeText("  ", input.selectionStart, input.selectionEnd, "end");
        paintInput();
        schedule();
      }
    }
  });

  // Files: "Open file" or dropped on the editor, read locally.
  const file = $("pg-file");
  $("pg-open").addEventListener("click", () => file.click());
  file.addEventListener("change", () => {
    openFile(file.files[0]);
    file.value = "";
  });
  const editor = $("pg-editor");
  let depth = 0;
  const hasFile = (e) => [...(e.dataTransfer?.types ?? [])].includes("Files");
  editor.addEventListener("dragenter", (e) => {
    if (!hasFile(e)) return;
    e.preventDefault();
    depth++;
    editor.classList.add("drop");
  });
  editor.addEventListener("dragover", (e) => {
    if (!hasFile(e)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
  });
  editor.addEventListener("dragleave", () => {
    if (--depth <= 0) {
      depth = 0;
      editor.classList.remove("drop");
    }
  });
  editor.addEventListener("drop", (e) => {
    if (!hasFile(e)) return;
    e.preventDefault();
    depth = 0;
    editor.classList.remove("drop");
    openFile(e.dataTransfer.files[0]);
  });

  formatter.readyHandlers.push(run);
  formatter.failHandlers.push(() => showError(LOAD_ERROR));
  paintInput();
}

// ---- install tabs -------------------------------------------------------------

/** One panel at a time, chosen by tabs that are built from the panel titles;
 * without scripting every panel is shown. Preselected: Homebrew on macOS and
 * Linux, Cargo elsewhere. */
function installTabs() {
  const box = $("inst");
  if (!box) return;
  const panels = [...box.querySelectorAll(".inst-panel")];
  const list = document.createElement("div");
  list.className = "inst-tabs";
  list.setAttribute("role", "tablist");
  list.setAttribute("aria-label", "Ways to install origami");
  const tabs = panels.map((p) => {
    const t = document.createElement("button");
    t.type = "button";
    t.id = `${p.id}-tab`;
    t.setAttribute("role", "tab");
    t.setAttribute("aria-controls", p.id);
    t.textContent = p.querySelector("h3").textContent;
    p.setAttribute("role", "tabpanel");
    p.setAttribute("aria-labelledby", t.id);
    list.append(t);
    return t;
  });
  const select = (i) => {
    tabs.forEach((t, j) => {
      t.setAttribute("aria-selected", String(i === j));
      t.tabIndex = i === j ? 0 : -1;
    });
    panels.forEach((p, j) => (p.hidden = i !== j));
  };
  tabKeys(tabs, select);
  const platform = (navigator.userAgentData?.platform || navigator.platform || "").toLowerCase();
  const ua = navigator.userAgent.toLowerCase();
  const unix = /mac|linux/.test(platform) && !/android|iphone|ipad/.test(ua);
  select(unix ? 0 : 1);
  box.prepend(list);
  box.classList.add("tabbed");
}

// ---- diagrams that scroll sideways get a tab stop -------------------------

function scrollers() {
  const ro = new ResizeObserver((entries) => {
    for (const { target } of entries) {
      const scrolls = target.scrollWidth > target.clientWidth + 1;
      if (scrolls) {
        target.tabIndex = 0;
        target.setAttribute("role", "region");
        target.setAttribute("aria-label", target.querySelector("img")?.alt.split(":")[0] || "Diagram");
      } else {
        target.removeAttribute("tabindex");
        target.removeAttribute("role");
        target.removeAttribute("aria-label");
      }
    }
  });
  for (const el of document.querySelectorAll("[data-scroll]")) ro.observe(el);
}

heroStart();
playground();
installTabs();
scrollers();
formatter.readyHandlers.push(() => {
  heroActivate();
  heroFormat();
});
try {
  startWorker();
} catch {
  formatter.failed = true;
  for (const f of formatter.failHandlers) f();
}
craneLoad();
