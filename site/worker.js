// Web Worker for the website: runs origami (WebAssembly, see
// crates/origami_wasm) off the main thread, so a slow input cannot freeze the
// page. app.js sends the requests of the journey, the playground and the
// speed measurement, and terminates and restarts this worker when a run takes
// too long.
//
// Messages in:  { id, op, source, opts: { cds, width, lower, compact }, runs }
//   op "review" (default): format and explain, the playground's request
//   op "journey": format, then lex the input and the output (the self-check's
//                 tokens), and time the formatting
//   op "lex": the tokens of `source`
//   op "bench": format `source` `runs` times, in batches, and return the time
//               of one call per batch
// Messages out: { type: "ready" } | { type: "load-error" }
//               { type: "result", id, result } | { type: "panic", id }

let wasm = null;

async function loadWasm() {
  const res = await fetch("origami.wasm");
  if (!res.ok) throw new Error(`origami.wasm: HTTP ${res.status}`);
  const { instance } = await WebAssembly.instantiate(await res.arrayBuffer(), {});
  return instance.exports;
}

/** Calls an export that takes a source buffer and returns JSON. */
function call(fn, source, ...args) {
  const bytes = new TextEncoder().encode(source);
  const ptr = wasm.alloc(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  const res = wasm[fn](ptr, bytes.length, ...args);
  const len = wasm.result_len();
  const json = new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, res, len));
  wasm.dealloc(res, len);
  return JSON.parse(json);
}

function review(source, opts) {
  return call("review", source, opts.cds ? 1 : 0, opts.width, 2, opts.lower ? 1 : 0, opts.compact ? 1 : 0);
}

function lex(source, cds) {
  return call("lex", source, cds ? 1 : 0);
}

const ops = {
  review({ source, opts }) {
    const t0 = performance.now();
    const result = review(source, opts);
    result.ms = performance.now() - t0;
    return result;
  },
  journey({ source, opts }) {
    const t0 = performance.now();
    const result = review(source, opts);
    const ms = performance.now() - t0;
    return {
      review: result,
      ms,
      lexIn: lex(source, opts.cds),
      lexOut: result.output == null ? null : lex(result.output, opts.cds),
    };
  },
  lex({ source, opts }) {
    return lex(source, opts?.cds);
  },
  bench({ source, opts, runs = 200 }) {
    // A few runs first, so the timing starts warm; then batches of 10 calls,
    // because the clock in a worker is coarse (0.1 ms).
    for (let i = 0; i < 5; i++) review(source, opts);
    const batch = 10;
    const perCall = [];
    for (let b = 0; b < Math.ceil(runs / batch); b++) {
      const t0 = performance.now();
      for (let i = 0; i < batch; i++) review(source, opts);
      perCall.push((performance.now() - t0) / batch);
    }
    return { perCall, calls: perCall.length * batch };
  },
};

const ready = loadWasm().then(
  (exports) => {
    wasm = exports;
    // A few runs first, so the times the page shows are of a warm formatter,
    // as in an editor that has formatted a file before.
    for (let i = 0; i < 3; i++) review("CLASS zcl_a IMPLEMENTATION.\n  METHOD a.\n    lv_a = 1.\n  ENDMETHOD.\nENDCLASS.\n", { cds: false, width: 120, lower: false, compact: false });
    postMessage({ type: "ready" });
  },
  () => postMessage({ type: "load-error" }),
);

onmessage = async ({ data }) => {
  await ready;
  if (!wasm) return;
  const { id } = data;
  try {
    const result = (ops[data.op] ?? ops.review)(data);
    postMessage({ type: "result", id, result });
  } catch {
    // A panic aborts the instance; start a fresh one.
    try {
      wasm = await loadWasm();
    } catch {
      wasm = null;
    }
    postMessage({ type: "panic", id });
  }
};
