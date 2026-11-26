// The paper crane in the hero: the seven facets of docs/assets/logo.svg as
// rigid sheets of paper that fold about their shared edges (three.js r170,
// vendor/). app.js loads this module when the page is idle and ties the fold
// to the margin of the hero sheet: one line is almost flat, more line breaks
// fold the crane further. Decorative: the canvas is aria-hidden and the page
// works without it (app.js shows the logo instead).

import {
  BufferGeometry,
  CanvasTexture,
  Color,
  DirectionalLight,
  Float32BufferAttribute,
  Group,
  HemisphereLight,
  Matrix4,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  PerspectiveCamera,
  PlaneGeometry,
  RepeatWrapping,
  SRGBColorSpace,
  Scene,
  ShadowMaterial,
  Vector3,
  VSMShadowMap,
  WebGLRenderer,
} from "./vendor/three.module.min.js";

// ---- geometry ----------------------------------------------------------------

// The facets of logo.svg (256-unit canvas, y down), their color role, the
// facet they hang from and the edge they turn about (the hinge), and the
// angle in radians at the full fold. Each facet starts to fold at `from` and
// is folded at `to` (fractions of the fold), so the crane folds in stages:
// body, wings, neck and tail, head.
export const FACETS = [
  { id: "bodyL", role: "valley", pts: [[80.96, 175.49], [127.58, 148.58], [127.58, 202.41]],
    hinge: [[127.58, 202.41], [127.58, 148.58]], angle: -0.7, from: 0, to: 0.5 },
  { id: "bodyR", role: "ink", pts: [[132.58, 148.58], [179.2, 175.49], [132.58, 202.41]],
    hinge: [[132.58, 202.41], [132.58, 148.58]], angle: 0.7, from: 0, to: 0.5 },
  { id: "wingF", role: "crease", parent: "bodyL", pts: [[90.46, 164.24], [149.58, 61.84], [127.94, 142.6]],
    hinge: [[90.46, 164.24], [127.94, 142.6]], angle: 0.8, from: 0.1, to: 0.85 },
  { id: "wingB", role: "valley", parent: "bodyR", pts: [[133.0, 143.04], [156.91, 53.8], [156.91, 156.85]],
    hinge: [[133.0, 143.04], [156.91, 156.85]], angle: -0.75, from: 0.1, to: 0.85 },
  { id: "neck", role: "ink", parent: "bodyL", pts: [[47.14, 57.14], [77.79, 171.55], [104.37, 156.21]],
    hinge: [[77.79, 171.55], [104.37, 156.21]], angle: 0.3, from: 0.2, to: 0.9 },
  { id: "tail", role: "crease", parent: "bodyR", pts: [[183.53, 172.22], [235.29, 120.46], [164.59, 161.28]],
    hinge: [[164.59, 161.28], [183.53, 172.22]], angle: 0.3, from: 0.2, to: 0.9 },
  { id: "head", role: "valley", parent: "neck", pts: [[20.71, 83.57], [47.14, 57.14], [54.22, 83.57]],
    hinge: [[47.14, 57.14], [54.22, 83.57]], angle: -1.0, from: 0.35, to: 1 },
];

/** The resting view: turned by `yaw` (radians) to a three-quarter view. */
export const VIEW = { yaw: -0.3, pitch: 0.3 };

// logo units -> scene units: x centered on the body, y up from the body's tip.
const S = 1 / 100;
const v3 = ([x, y]) => new Vector3((x - 130.08) * S, (202.41 - y) * S, 0);
const THICK = 0.009; // paper, a little under 1% of the crane's width

/** A triangle with a front, a back and three edges, flat-shaded. */
function facetGeometry(pts) {
  const [a, b, c] = pts.map(v3);
  const h = THICK / 2;
  const up = (p) => [p.x, p.y, h];
  const dn = (p) => [p.x, p.y, -h];
  const pos = [];
  const uv = [];
  const tri = (p, q, r) => {
    for (const v of [p, q, r]) {
      pos.push(...v);
      uv.push(v[0] * 1.6, v[1] * 1.6);
    }
  };
  // Winding so that the front faces +z.
  const ccw = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x) > 0;
  const [p, q, r] = ccw ? [a, b, c] : [a, c, b];
  tri(up(p), up(q), up(r));
  tri(dn(p), dn(r), dn(q));
  for (const [s, t] of [[p, q], [q, r], [r, p]]) {
    tri(dn(s), dn(t), up(t));
    tri(dn(s), up(t), up(s));
  }
  const g = new BufferGeometry();
  g.setAttribute("position", new Float32BufferAttribute(pos, 3));
  g.setAttribute("uv", new Float32BufferAttribute(uv, 2));
  g.computeVertexNormals();
  return g;
}

/** Dashes (6 on, 4 off, as the crease lines in brand.md) along a segment,
 * as thin quads: WebGL draws lines only one device pixel wide. */
function creaseGeometry(p, q) {
  const a = v3(p);
  const b = v3(q);
  const d = b.clone().sub(a);
  const len = d.length();
  d.normalize();
  const n = new Vector3(-d.y, d.x, 0).multiplyScalar(0.0045);
  const pos = [];
  const z = THICK;
  for (let s = 0.02; s < len - 0.02; s += 0.06) {
    const e = Math.min(s + 0.036, len - 0.02);
    const s0 = a.clone().addScaledVector(d, s);
    const e0 = a.clone().addScaledVector(d, e);
    const c = [s0.clone().add(n), s0.clone().sub(n), e0.clone().sub(n), e0.clone().add(n)];
    for (const i of [0, 1, 2, 0, 2, 3]) pos.push(c[i].x, c[i].y, z);
  }
  const g = new BufferGeometry();
  g.setAttribute("position", new Float32BufferAttribute(pos, 3));
  return g;
}

/** The crease between a facet and its parent: the hinge moved half a gap
 * toward the parent (the facets of the logo sit 5 units apart). */
function creaseLine(f, parent) {
  const [p, q] = f.hinge;
  if (!parent) return [[130.08, p[1]], [130.08, q[1]]];
  const cx = parent.pts.reduce((s, v) => s + v[0], 0) / 3;
  const cy = parent.pts.reduce((s, v) => s + v[1], 0) / 3;
  let nx = -(q[1] - p[1]);
  let ny = q[0] - p[0];
  const l = Math.hypot(nx, ny);
  nx /= l;
  ny /= l;
  if ((cx - p[0]) * nx + (cy - p[1]) * ny < 0) {
    nx = -nx;
    ny = -ny;
  }
  const off = f.id === "head" ? 0 : 2.5;
  return [[p[0] + nx * off, p[1] + ny * off], [q[0] + nx * off, q[1] + ny * off]];
}

// ---- paper -------------------------------------------------------------------

/** A tileable paper texture drawn on a canvas: fine grain and a few long
 * fibres, grey around the middle, used as a bump map. */
function paperTexture() {
  const size = 256;
  const c = document.createElement("canvas");
  c.width = c.height = size;
  const g = c.getContext("2d");
  g.fillStyle = "rgb(128,128,128)";
  g.fillRect(0, 0, size, size);
  const img = g.getImageData(0, 0, size, size);
  let seed = 7;
  const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  for (let i = 0; i < img.data.length; i += 4) {
    const v = 128 + (rnd() - 0.5) * 34;
    img.data[i] = img.data[i + 1] = img.data[i + 2] = v;
  }
  g.putImageData(img, 0, 0);
  g.lineCap = "round";
  for (let i = 0; i < 260; i++) {
    const x = rnd() * size;
    const y = rnd() * size;
    const a = rnd() * Math.PI;
    const len = 6 + rnd() * 26;
    const light = rnd() > 0.5;
    g.strokeStyle = light ? "rgba(255,255,255,0.18)" : "rgba(0,0,0,0.16)";
    g.lineWidth = 0.6 + rnd() * 0.8;
    // Drawn three times, shifted by the tile size, so the tile is seamless.
    for (const dx of [-size, 0, size]) {
      for (const dy of [-size, 0, size]) {
        g.beginPath();
        g.moveTo(x + dx, y + dy);
        g.quadraticCurveTo(
          x + dx + Math.cos(a) * len * 0.5 + (rnd() - 0.5) * 4,
          y + dy + Math.sin(a) * len * 0.5 + (rnd() - 0.5) * 4,
          x + dx + Math.cos(a) * len,
          y + dy + Math.sin(a) * len,
        );
        g.stroke();
      }
    }
  }
  const t = new CanvasTexture(c);
  t.wrapS = t.wrapT = RepeatWrapping;
  return t;
}

// ---- motion helpers ------------------------------------------------------------

const ease = (t) => (t <= 0 ? 0 : t >= 1 ? 1 : 1 - (1 - t) ** 3);
const smooth = (a, b, x) => {
  const t = Math.min(1, Math.max(0, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};
/** Moves `v` toward `to` with a time constant of `tau` seconds. */
const approach = (v, to, dt, tau) => to + (v - to) * Math.exp(-dt / tau);

// ---- the scene -------------------------------------------------------------------

/**
 * Mounts the crane into `box`.
 * `target`: the fold to reach (0 flat, 1 folded). `moment`: the hero's load
 * moment ({ t0, crease, fold, foldFor } in ms) or null. `onFail`: called when
 * WebGL stops working; the crane is then gone.
 */
export function mount(box, { target = 0.7, moment = null, reduceMotion = false, onFail = () => {} }) {
  const renderer = new WebGLRenderer({ antialias: true, alpha: true, powerPreference: "low-power" });
  renderer.setPixelRatio(Math.min(devicePixelRatio || 1, 2));
  renderer.outputColorSpace = SRGBColorSpace;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = VSMShadowMap;
  const canvas = renderer.domElement;
  canvas.setAttribute("aria-hidden", "true");
  box.append(canvas);

  const scene = new Scene();
  const camera = new PerspectiveCamera(28, 1, 0.1, 50);

  const hemi = new HemisphereLight(0xffffff, 0xffffff, 1.6);
  scene.add(hemi);
  // The key light: soft, from the upper left and a little in front.
  const key = new DirectionalLight(0xffffff, 2.2);
  key.position.set(-3.2, 4.4, 3.4);
  key.target.position.set(0, 0.6, 0);
  key.castShadow = true;
  key.shadow.mapSize.set(1024, 1024);
  key.shadow.radius = 9;
  key.shadow.blurSamples = 16;
  key.shadow.bias = -0.0004;
  Object.assign(key.shadow.camera, { left: -2.2, right: 2.2, top: 2.2, bottom: -2.2, near: 1, far: 12 });
  scene.add(key, key.target);

  const ground = new Mesh(new PlaneGeometry(12, 12), new ShadowMaterial({ opacity: 0.2 }));
  ground.rotation.x = -Math.PI / 2;
  ground.receiveShadow = true;
  scene.add(ground);

  const bump = paperTexture();
  const materials = {
    ink: new MeshStandardMaterial({ roughness: 0.92, metalness: 0, bumpMap: bump, bumpScale: 0.35 }),
    valley: new MeshStandardMaterial({ roughness: 0.92, metalness: 0, bumpMap: bump, bumpScale: 0.35 }),
    crease: new MeshStandardMaterial({ roughness: 0.92, metalness: 0, bumpMap: bump, bumpScale: 0.35 }),
  };
  const dash = new MeshBasicMaterial({ transparent: true, opacity: 0, depthWrite: false });

  // The crane turns as one; the facets fold inside it.
  const crane = new Group();
  scene.add(crane);
  const byId = new Map(FACETS.map((f) => [f.id, f]));
  const parts = FACETS.map((f) => {
    const mesh = new Mesh(facetGeometry(f.pts), materials[f.role]);
    // Only the ground takes the shadow: one soft shadow, no self-shadowing
    // artefacts on the thin facets.
    mesh.castShadow = true;
    mesh.matrixAutoUpdate = false;
    crane.add(mesh);
    // One crease per hinge; the two halves of the body share theirs.
    let crease = null;
    if (f.id !== "bodyR") {
      const [p, q] = creaseLine(f, f.parent ? byId.get(f.parent) : null);
      crease = new Mesh(creaseGeometry(p, q), dash);
      crease.matrixAutoUpdate = false;
      crease.renderOrder = 1;
      crane.add(crease);
    }
    const a = v3(f.hinge[0]);
    const axis = v3(f.hinge[1]).sub(a).normalize();
    return { f, mesh, crease, a, axis, local: new Matrix4(), world: new Matrix4() };
  });
  const partOf = new Map(parts.map((p) => [p.f.id, p]));

  const IDENTITY = new Matrix4();
  const tmp = new Matrix4();
  const toOrigin = new Matrix4();
  const back = new Matrix4();
  /** Facet matrices for fold `f`: parent * (rotation about the own hinge). */
  function pose(fold) {
    for (const p of parts) {
      const k = ease((fold - p.f.from) / (p.f.to - p.f.from));
      toOrigin.makeTranslation(-p.a.x, -p.a.y, -p.a.z);
      back.makeTranslation(p.a.x, p.a.y, p.a.z);
      tmp.makeRotationAxis(p.axis, p.f.angle * k);
      p.local.copy(back).multiply(tmp).multiply(toOrigin);
      const parent = p.f.parent ? partOf.get(p.f.parent) : null;
      p.world.copy(parent ? parent.world : IDENTITY).multiply(p.local);
      p.mesh.matrix.copy(p.world);
      p.mesh.matrixWorldNeedsUpdate = true;
      // A crease lies on its parent: it is where the facet turns.
      if (p.crease) {
        p.crease.matrix.copy(parent ? parent.world : IDENTITY);
        p.crease.matrixWorldNeedsUpdate = true;
      }
    }
  }

  // ---- colors from the page's tokens (they switch with the color scheme) ----
  const dark = matchMedia("(prefers-color-scheme: dark)");
  function colors() {
    const css = getComputedStyle(document.documentElement);
    const get = (n) => new Color(css.getPropertyValue(n).trim());
    materials.ink.color.copy(get("--ink"));
    materials.valley.color.copy(get("--valley"));
    materials.crease.color.copy(get("--crease"));
    dash.color.copy(get("--valley"));
    ground.material.color.copy(dark.matches ? new Color(0x000000) : get("--ink"));
    ground.material.opacity = dark.matches ? 0.55 : 0.2;
    hemi.intensity = dark.matches ? 1.25 : 1.6;
    key.intensity = dark.matches ? 2.0 : 2.2;
    wake();
  }

  // ---- view ------------------------------------------------------------------
  const BASE_YAW = VIEW.yaw;
  const view = { yaw: 0, pitch: 0, pointerYaw: 0, pointerPitch: 0, dragYaw: 0, dragPitch: 0 };
  function size() {
    const w = box.clientWidth;
    const h = box.clientHeight;
    if (!w || !h) return;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    // Fit the crane (about 2.6 by 1.9 units with its shadow) into the box.
    const t = Math.tan((camera.fov * Math.PI) / 360);
    const dist = Math.max(0.95 / t, 1.3 / (t * camera.aspect));
    const pitch = VIEW.pitch;
    camera.position.set(0, 0.72 + Math.sin(pitch) * dist, Math.cos(pitch) * dist);
    camera.lookAt(0, 0.7, 0);
    camera.updateProjectionMatrix();
    wake();
  }

  // ---- state and loop --------------------------------------------------------
  let fold = moment ? 0 : target;
  let goal = target;
  let creaseShown = moment ? 0 : 1;
  let running = false;
  let onScreen = true;
  let last = 0;
  let first = true;
  const t0 = moment?.t0 ?? performance.now();
  // Arriving after the load moment: no fold to replay.
  const late = moment && performance.now() - t0 > moment.fold;
  if (late) {
    fold = target;
    creaseShown = 1;
  }

  function step(now) {
    const dt = Math.min(0.05, last ? (now - last) / 1000 : 0.016);
    last = now;
    const t = now - t0;
    let busy = false;
    if (moment && !late && t < moment.fold + moment.foldFor * 2.2) {
      // The load moment: flat, creases at `crease`, folded from `fold` on.
      creaseShown = smooth(moment.crease, moment.crease + 220, t);
      const k = (t - moment.fold) / (moment.foldFor * 2.2);
      fold = goal * ease(Math.max(0, k));
      busy = true;
    } else if (reduceMotion) {
      fold = goal;
    } else {
      const f = approach(fold, goal, dt, 0.16);
      busy = Math.abs(f - goal) > 0.0005;
      fold = busy ? f : goal;
    }
    pose(fold);
    // The creases show while the sheet is (nearly) flat.
    dash.opacity = creaseShown * (1 - smooth(0.04, 0.3, fold)) * 0.95;

    // Turning: toward the pointer, by dragging, and a slow float.
    const tau = 0.35;
    const yaw = view.pointerYaw + view.dragYaw;
    const pitch = view.pointerPitch + view.dragPitch;
    if (reduceMotion) {
      view.yaw = yaw;
      view.pitch = pitch;
    } else {
      view.yaw = approach(view.yaw, yaw, dt, drag ? 0.06 : tau);
      view.pitch = approach(view.pitch, pitch, dt, drag ? 0.06 : tau);
      busy ||= Math.abs(view.yaw - yaw) > 0.0005 || Math.abs(view.pitch - pitch) > 0.0005;
    }
    const s = now / 1000;
    const float = reduceMotion ? 0 : 1;
    crane.rotation.set(view.pitch, BASE_YAW + view.yaw + float * 0.035 * Math.sin(s * 0.6), 0);
    crane.position.y = float * 0.018 * (0.5 + 0.5 * Math.sin(s * 1.1));
    renderer.render(scene, camera);
    if (first) {
      first = false;
      canvas.classList.add("on");
    }
    return busy || !reduceMotion;
  }

  function frame(now) {
    if (!running) return;
    if (step(now) && onScreen && !document.hidden) requestAnimationFrame(frame);
    else {
      running = false;
      last = 0;
    }
  }
  function wake() {
    if (running || !onScreen || document.hidden || disposed) return;
    running = true;
    requestAnimationFrame(frame);
  }

  // ---- input -----------------------------------------------------------------
  let drag = null;
  const fine = matchMedia("(hover: hover) and (pointer: fine)");
  function onPointerMove(e) {
    if (drag && e.pointerId === drag.id) {
      const dx = e.clientX - drag.x;
      const dy = e.clientY - drag.y;
      if (!drag.on) {
        if (Math.hypot(dx, dy) < 6) return;
        // Mostly sideways: turn the crane. Otherwise let the page scroll.
        if (Math.abs(dx) < Math.abs(dy) * 1.2) {
          drag = null;
          return;
        }
        drag.on = true;
        canvas.setPointerCapture(e.pointerId);
        canvas.classList.add("grabbing");
      }
      view.dragYaw = Math.max(-0.9, Math.min(0.9, dx * 0.008));
      view.dragPitch = Math.max(-0.12, Math.min(0.3, dy * 0.003));
      wake();
      return;
    }
    if (reduceMotion || !fine.matches || e.pointerType !== "mouse") return;
    const r = canvas.getBoundingClientRect();
    const nx = (e.clientX - (r.left + r.width / 2)) / innerWidth;
    const ny = (e.clientY - (r.top + r.height / 2)) / innerHeight;
    view.pointerYaw = Math.max(-0.22, Math.min(0.22, nx * 0.4));
    view.pointerPitch = Math.max(-0.08, Math.min(0.08, ny * 0.14));
    wake();
  }
  function onPointerDown(e) {
    if (reduceMotion || (e.pointerType === "mouse" && e.button !== 0)) return;
    drag = { id: e.pointerId, x: e.clientX, y: e.clientY, on: false };
  }
  function onPointerUp(e) {
    if (!drag || e.pointerId !== drag.id) return;
    drag = null;
    canvas.classList.remove("grabbing");
    // It eases back to where it was.
    view.dragYaw = 0;
    view.dragPitch = 0;
    wake();
  }
  canvas.addEventListener("pointerdown", onPointerDown);
  addEventListener("pointermove", onPointerMove, { passive: true });
  addEventListener("pointerup", onPointerUp);
  addEventListener("pointercancel", onPointerUp);

  // ---- lifecycle -----------------------------------------------------------------
  let disposed = false;
  const ro = new ResizeObserver(size);
  ro.observe(box);
  const io = new IntersectionObserver(([e]) => {
    onScreen = e.isIntersecting;
    wake();
  });
  io.observe(box);
  const onVisibility = () => wake();
  document.addEventListener("visibilitychange", onVisibility);
  dark.addEventListener("change", colors);

  function dispose() {
    if (disposed) return;
    disposed = true;
    running = false;
    ro.disconnect();
    io.disconnect();
    document.removeEventListener("visibilitychange", onVisibility);
    dark.removeEventListener("change", colors);
    canvas.removeEventListener("pointerdown", onPointerDown);
    removeEventListener("pointermove", onPointerMove);
    removeEventListener("pointerup", onPointerUp);
    removeEventListener("pointercancel", onPointerUp);
    for (const p of parts) {
      p.mesh.geometry.dispose();
      p.crease?.geometry.dispose();
    }
    ground.geometry.dispose();
    ground.material.dispose();
    for (const m of Object.values(materials)) m.dispose();
    dash.dispose();
    bump.dispose();
    renderer.dispose();
    renderer.forceContextLoss();
    canvas.remove();
  }
  canvas.addEventListener("webglcontextlost", (e) => {
    e.preventDefault();
    dispose();
    onFail();
  });

  colors();
  size();
  pose(fold);
  wake();

  return {
    /** The fold for the current margin: 0 flat, 1 folded. */
    setFold(f) {
      goal = Math.max(0, Math.min(1, f));
      wake();
    },
    dispose,
  };
}
