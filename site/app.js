// origami website, the entry module: copy buttons, tabs, the playground, the
// install tabs, the live speed measurement, and the journey (journey.js),
// all served by one worker (format.js, worker.js).

import {
  $,
  REPO,
  RUN_TIMEOUT_MS,
  announce,
  cancelAnnounce,
  formatMs,
  formatter,
  paint,
  request,
  startWorker,
  submit,
} from "./format.js";
import { journey } from "./journey.js";

for (const a of document.querySelectorAll("[data-repo-path]")) {
  a.href = `${REPO}/blob/main/${a.dataset.repoPath}`;
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
  cancelAnnounce();
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
        target.setAttribute("aria-label", target.querySelector("title, img")?.textContent.split(":")[0] || "Diagram");
      } else {
        target.removeAttribute("tabindex");
        target.removeAttribute("role");
        target.removeAttribute("aria-label");
      }
    }
  });
  for (const el of document.querySelectorAll("[data-scroll]")) ro.observe(el);
}

// ---- speed: measured here ------------------------------------------------------

/** Formats the playground's code 200 times in the worker and reports the
 * median of one call, next to the published numbers. */
function speed() {
  const box = $("measure");
  if (!box) return;
  const button = $("measure-button");
  const out = $("measure-out");
  box.hidden = false;
  button.disabled = true;
  formatter.readyHandlers.push(() => (button.disabled = false));
  button.addEventListener("click", async () => {
    button.disabled = true;
    const source = input.value;
    const lines = source.split("\n").length;
    out.textContent = "Measuring…";
    try {
      const r = await request("bench", { op: "bench", source, opts: options(), runs: 200, timeout: 30000 });
      const sorted = [...r.perCall].sort((a, b) => a - b);
      const median = sorted[Math.floor(sorted.length / 2)];
      out.innerHTML = "";
      const strong = document.createElement("strong");
      strong.textContent = `${formatMs(median)} ms`;
      out.append(
        "In this browser: ",
        strong,
        ` per call, the median of ${r.calls} calls on the playground's ${lines} lines, in WebAssembly.`,
      );
      mineRow(median);
      announce(out.textContent);
    } catch {
      out.textContent = "The measurement did not finish. Try again, or with less code in the playground.";
    }
    button.disabled = false;
  });
}

/** A row for this browser in the published table, on the same scale. */
function mineRow(ms) {
  const body = document.querySelector(".bars tbody");
  let row = body.querySelector(".mine");
  if (!row) {
    row = document.createElement("tr");
    row.className = "mine";
    row.innerHTML = '<th scope="row">This browser, playground code</th><td class="num"></td><td class="num">–</td><td class="bar" aria-hidden="true"><span class="b"></span></td>';
    body.insertBefore(row, body.querySelector(".other"));
  }
  row.querySelector(".num").textContent = formatMs(ms);
  // The scale of the published bars: log10 from 0.063 ms (0) to 315 ms (100%).
  const pct = Math.max(1, Math.min(100, ((Math.log10(Math.max(ms, 0.01)) + 1.2) / 3.7) * 100));
  row.querySelector(".b").style.width = `${pct}%`;
}

journey();
playground();
installTabs();
scrollers();
speed();
startWorker();
