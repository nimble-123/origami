// The journey: one overlong ABAP statement through what origami does with
// it, driven by the scroll position (a rAF-throttled controller; native
// scrolling is never changed). Every state on the stage comes from origami
// in the worker: the tokens from its lexer, the layout and the note from
// `review`, the check from lexing the output again. The paper crane
// (crane.js) travels from the hero to the stage and folds along.
//
// Without scripting, or with reduced motion, the stage stays hidden and the
// steps show their figures from index.html instead.

import { $, REPO, clamp, formatMs, formatter, paint, reduceMotion, request, submit } from "./format.js";

const SOURCE =
  "DATA(ls_rule) = zcl_flight_rules=>get( iv_carrier = is_flight-carrid iv_date = sy-datum iv_mode = 'A' ).";
const OPTS = { cds: false, lower: false, compact: true };
const MIN = 40;
const MAX = 120;
const STEPS = ["Read", "Fold", "Check", "Explain", "Write"];
// The headline's load moment (style.css), which the crane follows.
const MOMENT = { crease: 250, fold: 950, foldFor: 760 };
// What check.rs reports for a unit the self-check left as written.
const W001 = "Unit left as written: the self-check found a layout change that would alter tokens or comments";

const ease = (t) => (t <= 0 ? 0 : t >= 1 ? 1 : 1 - (1 - t) ** 3);
const lerp = (a, b, t) => a + (b - a) * t;

/** Tokens of a lex result with their text and (line, column). The offsets
 * are bytes into UTF-8; the columns count characters. */
function placed(text, lexed) {
  const bytes = new TextEncoder().encode(text);
  const dec = new TextDecoder();
  const lineStarts = [0];
  for (let i = 0; i < bytes.length; i++) if (bytes[i] === 10) lineStarts.push(i + 1);
  return lexed.tokens.map((t) => {
    let y = 0;
    while (y + 1 < lineStarts.length && lineStarts[y + 1] <= t.start) y++;
    return {
      text: dec.decode(bytes.subarray(t.start, t.end)),
      cls: t.class,
      glued: t.glued,
      x: dec.decode(bytes.subarray(lineStarts[y], t.start)).length,
      y,
    };
  });
}

/** Compares two token lists the way the self-check does: same count, same
 * class and text, same glued flag. Returns the index of the first
 * difference, or -1. (No keyword changes case here: the call is written in
 * the case origami prints.) */
function firstMismatch(a, b) {
  const n = Math.max(a.length, b.length);
  for (let i = 0; i < n; i++) {
    const x = a[i];
    const y = b[i];
    if (!x || !y || x.text !== y.text || x.cls !== y.cls || x.glued !== y.glued) return i;
  }
  return -1;
}

/** How far the crane is folded for a layout: one line is almost flat, more
 * lines fold it further. */
function foldFor(w, lines) {
  const base = lines <= 1 ? 0.05 : lines <= 3 ? 0.62 : 0.84;
  return clamp(base + 0.18 * ((MAX - w) / (MAX - MIN)), 0, 1);
}

export function journey() {
  const section = $("journey");
  if (!section) return;
  const live = !reduceMotion;
  const st = {
    w: 72,
    data: null, // { inTokens, outTokens, review, ms, lines, mismatch }
    cache: new Map(),
    spans: [],
    step: -1,
    t: 0,
    tamper: null, // { mismatch, outTokens, kept }
    raf: 0,
    crane: null,
  };

  // ---- the crane: one canvas, moved from the hero to the stage -------------
  const float = document.createElement("div");
  float.className = "crane-float";
  float.setAttribute("aria-hidden", "true");
  const heroSlot = $("crane-hero");
  const stageSlot = $("crane-stage");
  const stage = $("stage");
  const wide = matchMedia("(min-width: 64rem)");
  let craneFold = 1;

  craneLoad();

  if (!live) return;

  section.classList.add("live");
  stage.hidden = false;
  $("tamper").hidden = false;
  const tokensBox = $("tokens");
  const canvas = $("code-canvas");
  const scroll = $("code-scroll");
  const margin = $("margin");
  const foot = $("stage-foot");
  const inputRow = $("stage-input");
  const links = $("stage-links");
  const steps = [...section.querySelectorAll(".step")];
  foot.textContent = "Loading origami…";

  // ---- width -------------------------------------------------------------
  const chOf = (el) => {
    const probe = document.createElement("span");
    probe.textContent = "0".repeat(20);
    probe.style.position = "absolute";
    probe.style.visibility = "hidden";
    el.append(probe);
    const ch = probe.getBoundingClientRect().width / 20;
    probe.remove();
    return ch || 8;
  };
  const fit = () => {
    const pad = parseFloat(getComputedStyle(scroll).paddingLeft) || 0;
    return Math.floor((scroll.clientWidth - 2 * pad - 26) / chOf(canvas));
  };
  function setWidth(w, { load = true } = {}) {
    w = clamp(Math.round(w), MIN, MAX);
    if (w === st.w && st.data) return;
    st.w = w;
    canvas.style.setProperty("--w", w);
    $("margin-width").textContent = w;
    margin.setAttribute("aria-valuenow", w);
    margin.setAttribute("aria-valuetext", `line width ${w}`);
    if (load) fetchLayout();
  }

  // ---- data from the worker ---------------------------------------------
  function fetchLayout() {
    const w = st.w;
    const cached = st.cache.get(w);
    if (cached) {
      st.data = cached;
      st.tamper = null;
      return schedule();
    }
    if (!formatter.ready) return;
    submit("journey", {
      op: "journey",
      source: `${SOURCE}\n`,
      opts: { ...OPTS, width: w },
      latestOnly: true,
      onResult: (msg) => {
        if (msg.type !== "result") return;
        const r = msg.result;
        if (r.review.output == null || r.lexIn.status !== "ok" || r.lexOut?.status !== "ok") {
          foot.textContent = "origami could not format the call here.";
          return;
        }
        const output = r.review.output.replace(/\n$/, "");
        const data = {
          review: r.review,
          ms: r.ms,
          output,
          inTokens: placed(SOURCE, r.lexIn),
          outTokens: placed(output, r.lexOut),
          lines: output.split("\n").length,
        };
        data.mismatch = firstMismatch(data.inTokens, data.outTokens);
        st.cache.set(w, data);
        if (st.w === w) {
          st.data = data;
          st.tamper = null;
          $("tamper-button").setAttribute("aria-pressed", "false");
          $("tamper-button").textContent = "What if a token changed?";
          build();
          schedule();
        }
      },
    });
  }

  /** Spans for the input tokens, once (the tokens never change). */
  function build() {
    if (st.spans.length) return;
    const toks = st.data.inTokens;
    st.spans = toks.map((t, k) => {
      const s = document.createElement("span");
      s.className = "tok";
      s.dataset.k = k;
      s.append(...paint(t.text, { upper: true }));
      tokensBox.append(s);
      return s;
    });
    const bracket = document.createElement("span");
    bracket.className = "bracket";
    tokensBox.append(bracket);
    inputRow.replaceChildren(
      ...toks.map((t) => {
        const s = document.createElement("span");
        s.className = "tok";
        s.textContent = t.text;
        s.style.setProperty("--x", t.x);
        return s;
      }),
    );
    tokensBox.classList.add("instant");
    requestAnimationFrame(() => tokensBox.classList.remove("instant"));
  }

  // ---- scroll progress -----------------------------------------------------
  function progress() {
    // The reading line: the middle of the screen beside the stage, or the
    // middle of what the pinned stage leaves free on a phone.
    const line = wide.matches ? innerHeight * 0.5 : innerHeight * 0.74;
    let s = -1;
    let t = 0;
    for (let i = 0; i < steps.length; i++) {
      const r = steps[i].getBoundingClientRect();
      if (r.top <= line) {
        s = i;
        t = clamp((line - r.top) / Math.max(1, r.height), 0, 1);
      }
    }
    return { s, t };
  }

  function schedule() {
    if (st.raf) return;
    st.raf = requestAnimationFrame(() => {
      st.raf = 0;
      render();
      travel();
    });
  }
  addEventListener("scroll", schedule, { passive: true });
  addEventListener("resize", schedule);

  // ---- rendering the stage -------------------------------------------------
  let lastLive = {};
  function say(id, text) {
    // Live regions only hear final values, not every frame.
    if (lastLive[id] === text) return;
    lastLive[id] = text;
    $(id).textContent = text;
  }

  function render() {
    const { s, t } = progress();
    const step = Math.max(0, s);
    section.dataset.step = String(step);
    steps.forEach((li, i) => li.classList.toggle("current", i === step));
    $("stage-step").innerHTML = `<b>${step + 1}</b>${STEPS[step]}`;
    margin.setAttribute("aria-disabled", String(!st.data));
    margin.tabIndex = st.data ? 0 : -1;
    margin.classList.toggle("asleep", step < 1);
    const P = s < 0 ? 0 : s + t;
    craneFold = stageFold(P);
    if (!st.data) return;
    const d = st.data;
    const N = d.inTokens.length;
    const tam = st.tamper;

    // Positions: on the long line (Read), folding (Fold), folded after.
    const k = s < 1 ? 0 : s === 1 ? ease(clamp(t * 1.8, 0, 1)) : 1;
    const keptAsWritten = tam?.kept;
    st.spans.forEach((span, i) => {
      const a = d.inTokens[i];
      const b = keptAsWritten ? a : (tam?.outTokens[i] ?? d.outTokens[i]) ?? a;
      span.style.setProperty("--x", lerp(a.x, b.x, k).toFixed(3));
      span.style.setProperty("--y", lerp(a.y, b.y, k).toFixed(3));
      // Read: boxes sweep in; literals, comments and pragmas are kept.
      const read = s === 0 && i < Math.ceil(t * 1.4 * N);
      span.classList.toggle("boxed", read && a.cls !== "literal" && a.cls !== "pragma");
      span.classList.toggle("kept", (s === 0 && read) && (a.cls === "literal" || a.cls === "pragma" || a.cls === "verbatim"));
      // Check: matched tokens get a valley line; a difference turns red.
      const swept = s === 2 ? Math.ceil(clamp(t * 1.6, 0, 1) * N) : s > 2 ? N : 0;
      const bad = tam && s >= 2 && i === tam.mismatch;
      span.classList.toggle("ok", s === 2 && i < swept && !(tam && i >= tam.mismatch));
      span.classList.toggle("bad", !!bad);
    });
    const lines = keptAsWritten ? 1 : k < 1 ? 1 + Math.round((d.lines - 1) * k) : d.lines;
    tokensBox.style.setProperty("--lines", Math.max(3, lines));
    tokensBox.style.setProperty("--cols", k < 0.5 || keptAsWritten ? SOURCE.length : Math.max(...d.output.split("\n").map((l) => l.length)));

    // The note's lines get a bracket (Explain).
    const note = d.review.notes.find((n) => n.text.includes("because")) ?? d.review.notes[0];
    const bracket = tokensBox.querySelector(".bracket");
    if (note && !keptAsWritten) {
      bracket.style.setProperty("--y", note.first - 1);
      bracket.style.setProperty("--n", note.last - note.first + 1);
    } else bracket.style.setProperty("--n", 0);

    // The foot line and the live texts of the steps.
    const literals = d.inTokens.filter((x) => x.cls === "literal").length;
    const readText = `${N} tokens; ${literals} ${literals === 1 ? "literal" : "literals"} copied as ${literals === 1 ? "it is" : "they are"}.`;
    say("read-live", readText);
    if (s <= 0) {
      foot.textContent = s < 0 ? `${SOURCE.length} characters on one line.` : readText;
    } else if (s === 1) {
      const longest = Math.max(...d.output.split("\n").map((l) => l.length));
      foot.textContent =
        d.lines === 1
          ? `Line width ${st.w}: it fits on one line (${longest} characters).`
          : `Line width ${st.w}: ${d.lines} lines, the longest ${longest} characters.`;
    } else if (s === 2) {
      checkFoot(N, t);
    } else if (s === 3) {
      noteFoot(note);
    } else {
      foot.textContent = tam?.kept
        ? `Written as it was, with W001. Took ${formatMs(d.ms)} ms in your browser.`
        : `Formatted and checked in ${formatMs(d.ms)} ms in your browser, then written.`;
    }
    say("write-live", `This run took ${formatMs(d.ms)} ms in your browser.`);
    links.classList.toggle("on", s === 2);
    if (s === 2) drawLinks(N, t);
    else links.replaceChildren();
  }

  function checkFoot(N, t) {
    const tam = st.tamper;
    if (tam) {
      const i = tam.mismatch;
      foot.replaceChildren();
      const a = document.createElement("span");
      a.textContent = `${i} of ${N} tokens identical; token ${i + 1} differs. `;
      const b = document.createElement("span");
      b.className = "bad";
      b.textContent = tam.kept ? `W001 ${W001}.` : "Checking…";
      foot.append(a, b);
      return;
    }
    const swept = Math.min(N, Math.ceil(clamp(t * 1.6, 0, 1) * N));
    const same = st.data.mismatch < 0 ? swept : Math.min(swept, st.data.mismatch);
    foot.textContent = `${same} of ${N} tokens identical.`;
    if (swept === N) say("check-live", `${same} of ${N} tokens identical.`);
  }

  function noteFoot(note) {
    foot.replaceChildren();
    if (!note) {
      foot.textContent = `No line break needed at width ${st.w}, so there is no note.`;
      return;
    }
    const mark = document.createElement("span");
    mark.className = "note-mark";
    mark.textContent = "↳ ";
    const a = document.createElement("a");
    a.href = `${REPO}/blob/main/docs/STYLE.md#:~:text=${encodeURIComponent(note.rule)}`;
    a.textContent = note.rule;
    foot.append(mark, a, note.text.slice(note.rule.length));
  }

  /** Lines from each input token (the small line above) to its output
   * token, drawn as far as the sweep has come. The ends are computed from
   * the tokens' columns, not from their moving boxes. */
  function drawLinks(N, t) {
    if (!wide.matches) return links.replaceChildren();
    const sheet = links.getBoundingClientRect();
    const rowBox = inputRow.getBoundingClientRect();
    const codeBox = tokensBox.getBoundingClientRect();
    const chIn = chOf(inputRow);
    const ch = chOf(tokensBox);
    const lh = parseFloat(getComputedStyle(st.spans[0]).lineHeight) || 21;
    const swept = Math.ceil(clamp(t * 1.6, 0, 1) * N);
    const tam = st.tamper;
    const outs = tam?.kept ? st.data.inTokens : (tam?.outTokens ?? st.data.outTokens);
    const cut = tam ? tam.mismatch : st.data.mismatch < 0 ? N : st.data.mismatch;
    const ns = "http://www.w3.org/2000/svg";
    const lines = [];
    for (let i = 0; i < Math.min(swept, N, cut + 1); i++) {
      const a = st.data.inTokens[i];
      const b = outs[i] ?? a;
      const l = document.createElementNS(ns, "line");
      l.setAttribute("x1", (rowBox.left + (a.x + a.text.length / 2) * chIn - sheet.left).toFixed(1));
      l.setAttribute("y1", (rowBox.bottom - sheet.top).toFixed(1));
      l.setAttribute("x2", (codeBox.left + (b.x + b.text.length / 2) * ch - sheet.left).toFixed(1));
      l.setAttribute("y2", (codeBox.top + b.y * lh + 4 - sheet.top).toFixed(1));
      if (i === cut && tam) l.classList.add("bad");
      lines.push(l);
    }
    links.replaceChildren(...lines);
  }

  // ---- the crane's fold along the steps -----------------------------------
  function stageFold(P) {
    const f = st.data ? foldFor(st.w, st.data.lines) : 0.62;
    if (P < 1) return 0.02;
    if (P < 2) return lerp(0.02, f, ease(P - 1));
    if (P < 3) return lerp(f, 0.92, P - 2);
    return lerp(0.92, 1, Math.min(1, P - 3));
  }

  // ---- the margin: drag and keys --------------------------------------------
  // Reached by keyboard before the Fold step: bring that step into view, so
  // the line folds as the width changes.
  margin.addEventListener("focus", () => {
    if (Number(section.dataset.step) < 1) $("step-fold").scrollIntoView({ block: "center", behavior: "instant" });
  });
  margin.addEventListener("keydown", (e) => {
    if (margin.getAttribute("aria-disabled") === "true") return;
    const step = { ArrowRight: 1, ArrowUp: 1, ArrowLeft: -1, ArrowDown: -1, PageUp: 10, PageDown: -10 }[e.key];
    let w = null;
    if (step) w = st.w + step;
    else if (e.key === "Home") w = MIN;
    else if (e.key === "End") w = MAX;
    if (w == null) return;
    e.preventDefault();
    setWidth(w);
    reveal();
  });
  function reveal() {
    const m = margin.getBoundingClientRect();
    const s = scroll.getBoundingClientRect();
    if (m.right > s.right) scroll.scrollLeft += m.right - s.right + 8;
    else if (m.left < s.left) scroll.scrollLeft -= s.left - m.left + 8;
  }
  let drag = null;
  const fromPointer = () => setWidth((drag.x - canvas.getBoundingClientRect().left) / drag.ch);
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
    if (margin.getAttribute("aria-disabled") === "true" || (e.pointerType === "mouse" && e.button !== 0)) return;
    margin.setPointerCapture(e.pointerId);
    margin.classList.add("dragging");
    drag = { id: e.pointerId, x: e.clientX, ch: chOf(canvas), edge: 0 };
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

  // ---- the tamper proof -------------------------------------------------------
  const tamperButton = $("tamper-button");
  tamperButton.addEventListener("click", async () => {
    if (!st.data) return;
    if (st.tamper) {
      st.tamper = null;
      tamperButton.setAttribute("aria-pressed", "false");
      tamperButton.textContent = "What if a token changed?";
      say("check-live", "The output is origami's again; the check passes.");
      return schedule();
    }
    // The bug: the space in front of sy-datum goes missing in the output.
    const out = st.data.output;
    const at = out.indexOf("sy-datum");
    if (at < 1 || out[at - 1] !== " ") return;
    const tampered = out.slice(0, at - 1) + out.slice(at);
    tamperButton.disabled = true;
    try {
      const lexed = await request("lex", { op: "lex", source: tampered, opts: OPTS });
      const outTokens = placed(tampered, lexed);
      const mismatch = firstMismatch(st.data.inTokens, outTokens);
      st.tamper = { mismatch, outTokens, kept: false };
      tamperButton.setAttribute("aria-pressed", "true");
      tamperButton.textContent = "Undo the change";
      schedule();
      const N = st.data.inTokens.length;
      say(
        "check-live",
        `The check found a difference: ${mismatch} of ${N} tokens identical, token ${mismatch + 1} differs. origami leaves the statement as written and reports W001.`,
      );
      // A moment later, the statement is printed as it was written.
      setTimeout(() => {
        if (st.tamper) {
          st.tamper.kept = true;
          schedule();
          // On a narrow sheet the long line runs out of view: show the token.
          const x = st.data.inTokens[mismatch]?.x ?? 0;
          requestAnimationFrame(() =>
            requestAnimationFrame(() => {
              scroll.scrollLeft = Math.max(0, x * chOf(canvas) - scroll.clientWidth / 2);
            }),
          );
        }
      }, 900);
    } catch {
      say("check-live", "The simulation could not run here.");
    }
    tamperButton.disabled = false;
  });

  // ---- start -------------------------------------------------------------------
  formatter.readyHandlers.push(() => {
    foot.textContent = "";
    // One run first, so the time the stage shows is of a warm run, like
    // every run after it in an editor.
    submit("journey", {
      op: "review",
      source: `${SOURCE}\n`,
      opts: { ...OPTS, width: st.w },
      onResult: () => fetchLayout(),
    });
  });
  formatter.failHandlers.push(() => {
    // Without origami, the steps show their figures again.
    section.classList.remove("live");
    stage.hidden = true;
    $("tamper").hidden = true;
  });
  requestAnimationFrame(() => {
    setWidth(Math.min(72, Math.max(MIN, fit())), { load: false });
    schedule();
  });
  new ResizeObserver(() => schedule()).observe(stage);

  // ---- the crane: loading and travel -------------------------------------------
  function craneLoad() {
    const still = (slot) => slot?.classList.add("still");
    const conn = navigator.connection;
    const fallback = () => {
      still(heroSlot);
      still(stageSlot);
      float.remove();
    };
    if (conn?.saveData || (navigator.deviceMemory && navigator.deviceMemory < 2)) return fallback();
    const probe = document.createElement("canvas");
    // three.js r170 needs WebGL 2.
    const gl = probe.getContext("webgl2");
    if (!gl) return fallback();
    gl.getExtension("WEBGL_lose_context")?.loseContext();
    document.body.append(float);
    sizeFloat();
    new ResizeObserver(sizeFloat).observe(heroSlot);
    const t0 = performance.now();
    const go = () =>
      import("./crane.js")
        .then((m) => {
          st.crane = m.mount(float, {
            target: 1,
            moment: reduceMotion ? null : { t0, ...MOMENT },
            reduceMotion,
            onFail: fallback,
          });
          travel();
        })
        .catch(fallback);
    requestAnimationFrame(() => setTimeout(() => ("requestIdleCallback" in window ? requestIdleCallback(go, { timeout: 400 }) : go()), 0));
    addEventListener("scroll", () => !live && travel(), { passive: true });
    addEventListener("resize", () => travel());
  }

  function sizeFloat() {
    const r = heroSlot.getBoundingClientRect();
    float.style.width = `${Math.round(r.width)}px`;
    float.style.height = `${Math.round(r.height)}px`;
    travel();
  }

  /** Places the canvas: over the hero slot, then, as the journey arrives,
   * along the way to the stage's slot, where it stays while the stage is
   * pinned. The fold goes from standing (hero) to the stage's fold. */
  function travel() {
    if (!float.isConnected) return;
    const h = heroSlot.getBoundingClientRect();
    let r = h;
    let u = 0;
    if (live && !stage.hidden) {
      const s = stageSlot.getBoundingClientRect();
      const grid = section.querySelector(".journey-grid").getBoundingClientRect();
      const pinAt = wide.matches ? 64 + 24 : 64;
      // 0 while the journey is a screen away, 1 when the stage is pinned.
      u = clamp(1 - (grid.top - pinAt) / (innerHeight * 0.85), 0, 1);
      const e = ease(u);
      // The slot's own aspect: the crane is scaled to fit and centered.
      const scale = Math.min(s.width / h.width, s.height / h.height);
      const tw = h.width * scale;
      const th = h.height * scale;
      const tx = s.left + (s.width - tw) / 2;
      const ty = s.top + (s.height - th) / 2;
      r = { left: lerp(h.left, tx, e), top: lerp(h.top, ty, e), width: lerp(h.width, tw, e) };
    }
    const scale = r.width / h.width || 1;
    float.style.transform = `translate(${r.left.toFixed(1)}px, ${r.top.toFixed(1)}px) scale(${scale.toFixed(4)})`;
    // Off screen: hidden, and crane.js stops drawing (IntersectionObserver).
    const bottom = r.top + h.height * scale;
    float.classList.toggle("hidden", bottom < 0 || r.top > innerHeight);
    float.classList.toggle("grab", u === 0);
    if (st.crane) st.crane.setFold(live ? lerp(1, craneFold, ease(u)) : 1, { immediate: live && u > 0 });
  }
}
