// The hero's editor window: the sample class as written on the left, folded
// by origami (in the worker) on the right. Scrolling lays the tilted window
// flat; the blue margin on the right pane sets the line width and origami
// formats the class again at that width.

import { $, clamp, formatMs, formatter, reduceMotion, submit } from "./format.js";
import { codeRows, inputRows } from "./render.js";
import { SAMPLE_WIDTH } from "./samples.js";

const MIN = 40;
const MAX = 120;

export function hero() {
  const stage = $("hero-stage");
  if (!stage) return;
  const keywords = new Set((self.KEYWORDS?.abap ?? "").split(" ").filter(Boolean));
  const out = $("hero-out");
  const inn = $("hero-in");
  const margin = $("margin");
  const canvas = $("hero-canvas");
  const scroll = $("hero-scroll");
  const status = $("hero-status");
  const st = { w: SAMPLE_WIDTH, source: null, cache: new Map(), ready: false };

  // ---- scroll: the window lies down flat as the page moves -----------------
  const heroEl = stage.closest(".hero");
  if (!reduceMotion) {
    let raf = 0;
    const tilt = () => {
      raf = 0;
      // Flat once the window's top has risen a third of the screen.
      const top = stage.getBoundingClientRect().top;
      const start = innerHeight * 0.85;
      const p = clamp((start - top) / (innerHeight * 0.55), 0, 1);
      stage.style.setProperty("--p", p.toFixed(3));
    };
    addEventListener("scroll", () => raf || (raf = requestAnimationFrame(tilt)), { passive: true });
    addEventListener("resize", () => raf || (raf = requestAnimationFrame(tilt)));
    tilt();
    // The light drifts only while the hero is on screen.
    new IntersectionObserver(([e]) => heroEl.classList.toggle("paused", !e.isIntersecting)).observe(heroEl);
  }

  // ---- the files' tabs -------------------------------------------------------
  const tabs = [$("tab-src"), $("tab-toml")];
  const panels = [$("hero-panes"), $("hero-toml")];
  const select = (i) => {
    tabs.forEach((t, j) => {
      t.setAttribute("aria-selected", String(i === j));
      t.tabIndex = i === j ? 0 : -1;
    });
    panels.forEach((p, j) => (p.hidden = i !== j));
  };
  tabs.forEach((t, i) => {
    t.addEventListener("click", () => select(i));
    t.addEventListener("keydown", (e) => {
      const j = { ArrowRight: 1, ArrowLeft: 0, Home: 0, End: 1 }[e.key];
      if (j == null) return;
      e.preventDefault();
      select(j);
      tabs[j].focus();
    });
  });

  // ---- formatting ---------------------------------------------------------------
  function show(r, w) {
    if (r.output == null) return;
    out.innerHTML = codeRows(r.output, keywords, { notes: r.notes });
    inn.innerHTML = inputRows(st.source, r.output, keywords);
    status.textContent = `Formatted at width ${w} in ${formatMs(r.ms)} ms`;
  }
  function format() {
    const w = st.w;
    const cached = st.cache.get(w);
    if (cached) return show(cached, w);
    if (!formatter.ready || st.source == null) return;
    submit("hero", {
      op: "review",
      source: st.source,
      opts: { cds: false, lower: false, compact: false, width: w },
      latestOnly: true,
      onResult: (msg) => {
        if (msg.type !== "result") return;
        st.cache.set(w, msg.result);
        if (st.w === w) show(msg.result, w);
      },
    });
  }
  let pending = 0;
  function setWidth(w) {
    w = clamp(Math.round(w), MIN, MAX);
    if (w === st.w) return;
    st.w = w;
    canvas.style.setProperty("--w", w);
    $("margin-width").textContent = w;
    $("toml-width").textContent = w;
    margin.setAttribute("aria-valuenow", w);
    margin.setAttribute("aria-valuetext", `line width ${w}`);
    // At most one request per frame while the margin moves.
    if (!pending) pending = requestAnimationFrame(() => ((pending = 0), format()));
  }

  // ---- the margin -------------------------------------------------------------------
  const chOf = () => {
    const probe = document.createElement("span");
    probe.textContent = "0".repeat(20);
    probe.className = "code";
    probe.style.position = "absolute";
    probe.style.visibility = "hidden";
    canvas.append(probe);
    const ch = probe.getBoundingClientRect().width / 20;
    probe.remove();
    return ch || 7.5;
  };
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
    const m = margin.getBoundingClientRect();
    const s = scroll.getBoundingClientRect();
    if (m.right > s.right) scroll.scrollLeft += m.right - s.right + 8;
    else if (m.left < s.left) scroll.scrollLeft -= s.left - m.left + 8;
  });
  let drag = null;
  const fromPointer = () => {
    const left = canvas.getBoundingClientRect().left + 14 + 4.2 * drag.ch;
    setWidth((drag.x - left) / drag.ch);
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
    if (margin.getAttribute("aria-disabled") === "true" || (e.pointerType === "mouse" && e.button !== 0)) return;
    margin.setPointerCapture(e.pointerId);
    margin.classList.add("dragging");
    drag = { id: e.pointerId, x: e.clientX, ch: chOf(), edge: 0 };
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

  // ---- start: the sample, then the margin once origami is ready -------------------
  margin.hidden = false;
  fetch("samples/zcl_flight_check.clas.abap")
    .then((r) => (r.ok ? r.text() : Promise.reject(new Error(String(r.status)))))
    .then((text) => {
      st.source = text;
      format();
    })
    .catch(() => {});
  const activate = () => {
    margin.setAttribute("aria-disabled", "false");
    $("hero-hint").hidden = false;
    format();
  };
  if (formatter.ready) activate();
  else formatter.readyHandlers.push(activate);
}
