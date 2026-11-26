// Shared by the journey and the playground: syntax colors, announcements and
// the one worker that runs origami (worker.js).

export const REPO = "https://github.com/nimble-123/origami";
export const $ = (id) => document.getElementById(id);
export const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
export const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

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
export function paint(text, { cds = false, upper = false } = {}) {
  const out = [];
  let pos = 0;
  const words = cds ? KEYWORDS.cds : KEYWORDS.abap;
  for (const m of text.matchAll(cds ? CDS_SYNTAX : ABAP_SYNTAX)) {
    let cls = m[1] ? "com" : m[2] ? "str" : m[3] ? "num-lit" : null;
    if (!cls) {
      const w = m[4];
      const kw = upper && !cds ? /^[A-Z][A-Z-]*[A-Z]$/.test(w) : words.has(w.toUpperCase());
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

// ---- announcements ----------------------------------------------------------

// A visually hidden live region: the page says what changed, but only for
// state changes the visitor asked for (a copy, an error, a finished run).
const announcer = $("announce");
let announceTimer = 0;
export function announce(message, delay = 0) {
  clearTimeout(announceTimer);
  // Emptied first, so the same message twice is announced twice.
  if (announcer) announcer.textContent = "";
  announceTimer = setTimeout(() => {
    if (announcer) announcer.textContent = message;
  }, delay);
}
export function cancelAnnounce() {
  clearTimeout(announceTimer);
}

// ---- the formatter: one worker for the whole page ----------------------------

// A run that takes longer than RUN_TIMEOUT_MS terminates the worker and starts
// a new one.
export const RUN_TIMEOUT_MS = 5000;

/** Requests by channel ("journey", "pg", "bench"): only the newest request
 * of a channel waits; the journey goes first, so dragging the margin stays
 * direct while the playground formats a large file. */
export const formatter = {
  worker: null,
  ready: false,
  failed: false,
  nextId: 0,
  inflight: null, // { id, timer, job, channel }
  queue: new Map(),
  readyHandlers: [],
  failHandlers: [],
};

function fail() {
  formatter.failed = true;
  for (const f of formatter.failHandlers) f();
}

export function startWorker() {
  formatter.ready = false;
  let mine;
  try {
    mine = new Worker("worker.js");
  } catch {
    fail();
    return;
  }
  formatter.worker = mine;
  mine.onmessage = ({ data }) => {
    if (mine !== formatter.worker) return;
    if (data.type === "ready") {
      formatter.ready = true;
      for (const f of formatter.readyHandlers) f();
      dispatch();
    } else if (data.type === "load-error") {
      fail();
    } else if (formatter.inflight && data.id === formatter.inflight.id) {
      const { timer, job, channel } = formatter.inflight;
      clearTimeout(timer);
      formatter.inflight = null;
      // A result that a newer request of its channel replaces is dropped.
      if (!(job.latestOnly && formatter.queue.has(channel))) job.onResult(data);
      dispatch();
    }
  };
  mine.onerror = fail;
}

export function submit(channel, job) {
  formatter.queue.set(channel, job);
  if (formatter.ready && !formatter.inflight) dispatch();
}

function dispatch() {
  if (!formatter.ready || formatter.inflight || !formatter.queue.size) return;
  const channel = formatter.queue.has("journey") ? "journey" : formatter.queue.keys().next().value;
  const job = formatter.queue.get(channel);
  formatter.queue.delete(channel);
  const id = formatter.nextId++;
  const timer = setTimeout(() => {
    // The worker is stuck in a long run: stop it and start a new one.
    formatter.worker.terminate();
    formatter.inflight = null;
    job.onTimeout?.();
    startWorker();
  }, job.timeout ?? RUN_TIMEOUT_MS);
  formatter.inflight = { id, timer, job, channel };
  formatter.worker.postMessage({ id, op: job.op, source: job.source, opts: job.opts, runs: job.runs });
}

/** A promise for one request; rejects when the run fails or times out. */
export function request(channel, job) {
  return new Promise((resolve, reject) => {
    submit(channel, {
      ...job,
      onResult: (data) => (data.type === "result" ? resolve(data.result) : reject(new Error("panic"))),
      onTimeout: () => reject(new Error("timeout")),
    });
  });
}

/** Milliseconds for people: one decimal below 10, "under 0.1" below that. */
export function formatMs(ms) {
  return ms < 0.1 ? "under 0.1" : ms < 10 ? ms.toFixed(1) : String(Math.round(ms));
}
