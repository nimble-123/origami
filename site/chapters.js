// The feature chapters: the self-check on one call (a cursor walks the
// tokens when the tile comes into view, and a simulated formatter bug is
// caught), and the terminal that types `origami --review` once. Every token
// comes from origami's lexer in the worker; the terminal shows the output
// site/fill.mjs put into the page (from site/samples/review.ansi).

import { $, formatter, reduceMotion, request } from "./format.js";
import { esc } from "./render.js";
import { CHECK_SOURCE, SAMPLE_WIDTH } from "./samples.js";

// What check.rs reports for a statement the self-check left as written.
const W001 = "Unit left as written: the self-check found a layout change that would alter tokens or comments";

/** Tokens of a lex result with their text and offsets (ASCII sample). */
const toks = (text, lexed) => lexed.tokens.map((t) => ({ text: text.slice(t.start, t.end), start: t.start, end: t.end, cls: t.class, glued: t.glued }));

/** The first token that differs, the way the self-check compares: kind,
 * text and the glued flag; -1 when all are the same. */
function firstMismatch(a, b) {
  const n = Math.max(a.length, b.length);
  for (let i = 0; i < n; i++) {
    const x = a[i];
    const y = b[i];
    if (!x || !y || x.text !== y.text || x.cls !== y.cls || x.glued !== y.glued) return i;
  }
  return -1;
}

/** `text` with each token wrapped in <u data-k>. */
function wrap(text, tokens) {
  let html = "";
  let pos = 0;
  tokens.forEach((t, k) => {
    html += esc(text.slice(pos, t.start)) + `<u data-k="${k}">${esc(t.text)}</u>`;
    pos = t.end;
  });
  return html + esc(text.slice(pos));
}

export function chapters() {
  check();
  terminal();
}

function check() {
  const tile = document.querySelector(".tile-check");
  if (!tile) return;
  const inRow = $("check-in");
  const outEl = $("check-out");
  const foot = $("check-foot");
  const link = $("check-link");
  const now = $("count-now");
  const label = $("count-label");
  const button = $("tamper-button");
  const st = { data: null, tamper: null, cur: -1, raf: 0, seen: false };

  function build() {
    const d = st.data;
    inRow.innerHTML = `<div class="check-in-track" id="check-track">${wrap(CHECK_SOURCE, d.inTokens)}</div>`;
    const showAsWritten = st.tamper?.kept;
    outEl.innerHTML = showAsWritten ? wrap(CHECK_SOURCE, d.inTokens) : wrap(st.tamper?.text ?? d.output, st.tamper?.outTokens ?? d.outTokens);
    outEl.classList.remove("done");
  }

  /** Marks the tokens up to the cursor in both rows, one link between the
   * pair under it, and the counts. */
  function paintCursor() {
    const d = st.data;
    const N = d.inTokens.length;
    const tam = st.tamper;
    const stop = tam ? tam.mismatch : N;
    const cur = Math.min(st.cur, stop);
    const ins = inRow.querySelectorAll("u");
    const outs = outEl.querySelectorAll("u");
    const mark = (list, i, cls) => list[i]?.classList.add(cls);
    for (const list of [ins, outs]) for (const u of list) u.className = "";
    for (let i = 0; i < Math.min(cur, N); i++) {
      mark(ins, i, "ok");
      if (!tam?.kept) mark(outs, i, "ok");
    }
    const atBad = tam && cur >= tam.mismatch;
    if (cur < N || atBad) {
      mark(ins, cur, atBad ? "bad" : "cur");
      mark(outs, cur, atBad ? "bad" : "cur");
    }
    // The input row follows the cursor, starting at a token.
    const track = $("check-track");
    const u = ins[Math.min(cur, N - 1)];
    let shift = 0;
    if (u && cur < N) {
      const want = u.offsetLeft - inRow.clientWidth / 2 + u.offsetWidth / 2;
      const max = track.scrollWidth - inRow.clientWidth;
      // The row starts at a word: a token with a space (or nothing) before it.
      const starts = d.inTokens.map((t, i) => (t.start === 0 || CHECK_SOURCE[t.start - 1] === " " ? ins[i].offsetLeft : null));
      shift = starts.filter((x) => x != null && x <= Math.min(want, max)).pop() ?? 0;
    }
    track.style.setProperty("--shift", `${-shift}px`);
    // One line between the pair under the cursor.
    link.replaceChildren();
    const a = ins[cur];
    const b = outs[cur];
    if (a && b && (cur < N || atBad) && !tam?.kept) {
      const box = link.getBoundingClientRect();
      const row = inRow.getBoundingClientRect();
      const bb = b.getBoundingClientRect();
      const x1 = row.left + a.offsetLeft - shift + a.offsetWidth / 2 - box.left;
      if (x1 > row.left - box.left && x1 < row.right - box.left) {
        const l = document.createElementNS("http://www.w3.org/2000/svg", "line");
        l.setAttribute("x1", x1.toFixed(1));
        l.setAttribute("y1", (row.bottom - box.top).toFixed(1));
        l.setAttribute("x2", (bb.left + bb.width / 2 - box.left).toFixed(1));
        l.setAttribute("y2", (bb.top - box.top).toFixed(1));
        if (atBad) l.classList.add("bad");
        link.append(l);
      }
    }
    // The path through the pipeline: written, or kept as written.
    document.querySelector(".tile-pipe")?.classList.toggle("caught", Boolean(tam?.kept));
    // The counts.
    const same = Math.min(cur, stop);
    now.textContent = String(same);
    if (atBad) {
      foot.innerHTML = `${same} of ${N} tokens identical; token ${same + 1} differs. <span class="bad">${tam.kept ? `W001 ${esc(W001)}.` : "The statement goes back to how it was written…"}</span>`;
      label.textContent = `token ${same + 1} differs`;
      label.classList.add("bad");
    } else if (cur >= N) {
      foot.textContent = `${N} of ${N} tokens identical. The file can be written.`;
      label.textContent = "tokens identical";
      label.classList.remove("bad");
      outEl.classList.add("done");
    } else {
      foot.textContent = `${same} of ${N} tokens identical. Token ${cur + 1}: ${d.inTokens[cur].text}`;
      label.textContent = "tokens identical";
      label.classList.remove("bad");
    }
  }

  /** Walks the cursor to the end (or to a difference), 70 ms a token. */
  function walk(from = 0) {
    cancelAnimationFrame(st.raf);
    const N = st.data.inTokens.length;
    const stop = st.tamper ? st.tamper.mismatch : N;
    if (reduceMotion) {
      st.cur = stop;
      paintCursor();
      return afterStop();
    }
    const t0 = performance.now();
    const step = (t) => {
      st.cur = Math.min(stop, from + Math.floor((t - t0) / 70));
      paintCursor();
      if (st.cur < stop) st.raf = requestAnimationFrame(step);
      else afterStop();
    };
    st.raf = requestAnimationFrame(step);
  }
  function afterStop() {
    const tam = st.tamper;
    if (!tam || tam.kept) return;
    setTimeout(() => {
      if (st.tamper !== tam) return;
      tam.kept = true;
      build();
      paintCursor();
    }, reduceMotion ? 0 : 900);
  }

  button.addEventListener("click", async () => {
    if (!st.data) return;
    if (st.tamper) {
      st.tamper = null;
      button.setAttribute("aria-pressed", "false");
      button.textContent = "What if a token changed?";
      build();
      walk(0);
      return;
    }
    // The bug: the space in front of sy-datum goes missing in the output.
    const out = st.data.output;
    const at = out.indexOf("sy-datum");
    if (at < 1 || out[at - 1] !== " ") return;
    const text = out.slice(0, at - 1) + out.slice(at);
    button.disabled = true;
    try {
      const lexed = await request("check-lex", { op: "lex", source: text, opts: { cds: false } });
      const outTokens = toks(text, lexed);
      st.tamper = { text, outTokens, mismatch: firstMismatch(st.data.inTokens, outTokens), kept: false };
      button.setAttribute("aria-pressed", "true");
      button.textContent = "Undo the change";
      build();
      walk(0);
    } catch {
      foot.textContent = "The simulation could not run here.";
    }
    button.disabled = false;
  });

  const start = () => {
    request("check", { op: "journey", source: `${CHECK_SOURCE}\n`, opts: { cds: false, lower: false, compact: false, width: SAMPLE_WIDTH } })
      .then((r) => {
        if (r.review.output == null || r.lexIn.status !== "ok" || r.lexOut.status !== "ok") return;
        const output = r.review.output.replace(/\n$/, "");
        st.data = { output, inTokens: toks(CHECK_SOURCE, r.lexIn), outTokens: toks(r.review.output, r.lexOut) };
        $("tamper").hidden = false;
        build();
        st.cur = st.seen ? 0 : -1;
        if (st.seen) walk(0);
        else paintCursor();
      })
      .catch(() => {});
  };
  // The cursor walks once, when the tile comes into view.
  new IntersectionObserver(
    ([e], io) => {
      if (!e.isIntersecting) return;
      io.disconnect();
      st.seen = true;
      if (st.data) walk(0);
    },
    { threshold: 0.5 },
  ).observe(tile);
  if (formatter.ready) start();
  else formatter.readyHandlers.push(start);
}

/** The terminal types the command, then the output appears line by line.
 * Once; without motion (or scripting) everything is there from the start. */
function terminal() {
  const cmd = $("term-cmd");
  const out = $("term-out");
  if (!cmd || !out || reduceMotion) return;
  const text = cmd.textContent;
  const lines = out.innerHTML.split("\n");
  cmd.textContent = "";
  out.innerHTML = "";
  const caret = document.createElement("span");
  caret.className = "caret";
  cmd.after(caret);
  new IntersectionObserver(
    ([e], io) => {
      if (!e.isIntersecting) return;
      io.disconnect();
      let i = 0;
      const type = () => {
        cmd.textContent = text.slice(0, ++i);
        if (i < text.length) return setTimeout(type, 28);
        caret.remove();
        let n = 0;
        const line = () => {
          out.innerHTML = lines.slice(0, ++n).join("\n");
          if (n < lines.length) setTimeout(line, 45);
        };
        setTimeout(line, 260);
      };
      setTimeout(type, 300);
    },
    { threshold: 0.4 },
  ).observe(cmd.closest(".window"));
}
