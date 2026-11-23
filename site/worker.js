// Web Worker for the playground: runs origami (WebAssembly, see
// crates/origami_wasm) off the main thread, so a slow input cannot freeze the
// page. app.js terminates and restarts this worker when a run takes too long.
//
// Messages in:  { id, source, opts: { cds, width, lower } }
// Messages out: { type: "ready" } | { type: "load-error" }
//               { type: "result", id, result } | { type: "panic", id }

let wasm = null;

async function loadWasm() {
  const res = await fetch("origami.wasm");
  if (!res.ok) throw new Error(`origami.wasm: HTTP ${res.status}`);
  const { instance } = await WebAssembly.instantiate(await res.arrayBuffer(), {});
  return instance.exports;
}

function review(source, opts) {
  const bytes = new TextEncoder().encode(source);
  const ptr = wasm.alloc(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  const res = wasm.review(ptr, bytes.length, opts.cds ? 1 : 0, opts.width, 2, opts.lower ? 1 : 0);
  const len = wasm.result_len();
  const json = new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, res, len));
  wasm.dealloc(res, len);
  return JSON.parse(json);
}

const ready = loadWasm().then(
  (exports) => {
    wasm = exports;
    postMessage({ type: "ready" });
  },
  () => postMessage({ type: "load-error" }),
);

onmessage = async ({ data: { id, source, opts } }) => {
  await ready;
  if (!wasm) return;
  const t0 = performance.now();
  try {
    const result = review(source, opts);
    result.ms = Math.max(1, Math.round(performance.now() - t0));
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
