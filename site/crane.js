// The paper crane in the closing spotlight: the seven facets of
// docs/assets/logo.svg as rigid sheets of paper that fold about their shared
// edges (three.js r170, vendor/), with its reflection on the floor. app.js
// loads this module as the closing section comes near; the crane folds up
// from a flat sheet once it is in view. Decorative: the canvas is
// aria-hidden and the page works without it (app.js shows the logo).

import {
  BufferGeometry,
  Color,
  DirectionalLight,
  Float32BufferAttribute,
  Group,
  HemisphereLight,
  Matrix4,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  MirroredRepeatWrapping,
  NoColorSpace,
  PerspectiveCamera,
  PlaneGeometry,
  PointLight,
  Scene,
  ShadowMaterial,
  SRGBColorSpace,
  TextureLoader,
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
    hinge: [[127.58, 202.41], [127.58, 148.58]], angle: -0.7, from: 0.08, to: 0.5 },
  { id: "bodyR", role: "ink", pts: [[132.58, 148.58], [179.2, 175.49], [132.58, 202.41]],
    hinge: [[132.58, 202.41], [132.58, 148.58]], angle: 0.7, from: 0.08, to: 0.5 },
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
      uv.push(v[0] * 0.5, v[1] * 0.5);
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
  const n = new Vector3(-d.y, d.x, 0).multiplyScalar(0.0085);
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

/** The paper fibre (docs/assets/paper-fibre.webp, see brand.md): a bump map,
 * and a faint grain in the color. Mirrored, so the tile has no seam. The
 * crane is drawn without it until it has loaded. */
function paperTexture(onLoad) {
  const url = new URL("assets/paper-fibre.webp", document.baseURI).href;
  return new TextureLoader().load(
    url,
    (t) => {
      t.wrapS = t.wrapT = MirroredRepeatWrapping;
      t.colorSpace = NoColorSpace;
      onLoad(t);
    },
    undefined,
    () => {},
  );
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
export function mount(box, { reduceMotion = false, onFail = () => {} }) {
  const target = 1;
  const moment = null;
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
  // Close to the crane, so light falls off across each facet, as on paper
  // by a window.
  const near = new PointLight(0xffffff, 5, 0, 2);
  near.position.set(-1.7, 2.1, 2.3);
  scene.add(near);

  const ground = new Mesh(new PlaneGeometry(12, 12), new ShadowMaterial({ opacity: 0.2 }));
  ground.rotation.x = -Math.PI / 2;
  ground.receiveShadow = true;
  scene.add(ground);

  const paper = () => new MeshStandardMaterial({ roughness: 0.9, metalness: 0 });
  const materials = { ink: paper(), valley: paper(), crease: paper() };
  const bump = paperTexture((t) => {
    for (const m of Object.values(materials)) {
      m.bumpMap = t;
      m.bumpScale = 0.6;
      m.map = t;
      m.needsUpdate = true;
    }
    dirty = true;
    wake();
  });
  const dash = new MeshBasicMaterial({ transparent: true, opacity: 0, depthWrite: false });

  // The crane turns as one; the facets fold inside it.
  const crane = new Group();
  scene.add(crane);
  // The reflection on the floor: the same facets, mirrored below the ground,
  // faint (its fade is a mask on the canvas, style.css).
  const mirrorRoot = new Group();
  mirrorRoot.scale.y = -1;
  const mirror = new Group();
  mirrorRoot.add(mirror);
  scene.add(mirrorRoot);
  const ghost = (m) => {
    const g = m.clone();
    g.transparent = true;
    g.opacity = 0.16;
    g.depthWrite = false;
    return g;
  };
  // Flat, the facets are one sheet of paper: a sheet behind them closes the
  // gaps, and the facets take the paper's color; folding, the sheet goes
  // and the facets take their own colors.
  const backing = new Mesh(
    new PlaneGeometry(2.39, 1.73),
    new MeshStandardMaterial({ roughness: 0.9, metalness: 0, transparent: true, opacity: 0, depthWrite: false }),
  );
  backing.position.set(-0.021, 0.743, -THICK * 0.8);
  crane.add(backing);
  const base = { ink: new Color(), valley: new Color(), crease: new Color(), paper: new Color() };
  function tint(fold) {
    const flat = 1 - smooth(0, 0.22, fold);
    for (const role of ["ink", "valley", "crease"]) materials[role].color.copy(base[role]).lerp(base.paper, flat * 0.5);
    // Gone before the first facet turns, so none can pass behind it.
    const sheet = 1 - smooth(0.01, 0.08, fold);
    backing.material.opacity = sheet;
    backing.visible = sheet > 0.01;
    for (const p of parts) p.image.material.color.copy(p.mesh.material.color);
  }
  const byId = new Map(FACETS.map((f) => [f.id, f]));
  const parts = FACETS.map((f) => {
    const mesh = new Mesh(facetGeometry(f.pts), materials[f.role]);
    // Only the ground takes the shadow: one soft shadow, no self-shadowing
    // artefacts on the thin facets.
    mesh.castShadow = true;
    mesh.matrixAutoUpdate = false;
    crane.add(mesh);
    const image = new Mesh(mesh.geometry, ghost(materials[f.role]));
    image.matrixAutoUpdate = false;
    mirror.add(image);
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
    return { f, mesh, image, crease, a, axis, local: new Matrix4(), world: new Matrix4() };
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
      p.image.matrix.copy(p.world);
      p.image.matrixWorldNeedsUpdate = true;
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
    // The logo's dark tones: ink, valley blue, slate for the crease facets.
    base.ink.copy(get("--text"));
    base.valley.copy(get("--blue"));
    base.crease.copy(new Color(0x3b4261));
    // The flat sheet: a lighter slate, so it stands off the dark ground.
    base.paper.copy(new Color(0x3b4261).lerp(new Color(0x737aa2), 0.35));
    backing.material.color.copy(base.paper);
    tint(fold);
    dash.color.copy(get("--blue"));
    for (const p of parts) p.image.material.color.copy(p.mesh.material.color);
    ground.material.color.copy(new Color(0x000000));
    ground.material.opacity = 0.5;
    hemi.intensity = 1.0;
    key.intensity = 1.6;
    near.intensity = 5;
    dirty = true;
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
    camDist = Math.max(0.95 / t, 1.3 / (t * camera.aspect));
    camera.updateProjectionMatrix();
    aim(lieOf(fold));
    dirty = true;
    wake();
  }
  let camDist = 6;
  /** Flat, the sheet lies on the table and the camera looks down on it;
   * folding, it stands up. */
  const lieOf = (f) => 1 - smooth(0, 0.35, f);
  function aim(lie) {
    const pitch = VIEW.pitch + lie * 0.5;
    const look = new Vector3(0, 0.7 * (1 - lie) + 0.05 * lie, -0.72 * lie);
    camera.position.set(0, look.y + Math.sin(pitch) * camDist, look.z + Math.cos(pitch) * camDist);
    camera.lookAt(look);
  }

  // ---- state and loop --------------------------------------------------------
  // Flat until the section is in view, then it folds up once.
  let fold = reduceMotion ? target : 0;
  let goal = fold;
  let unfoldAt = null;
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

  let dirty = true;
  function step(now) {
    if (unfoldAt != null) {
      const k = Math.min(1, (now - unfoldAt) / 1900);
      goal = fold = k;
      if (k >= 1) unfoldAt = null;
      dirty = true;
    }
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
    tint(fold);
    // The creases show while the sheet is (nearly) flat.
    dash.opacity = creaseShown * (1 - smooth(0.04, 0.3, fold)) * 0.95;

    // Turning: toward the pointer and by dragging. At rest it is still and
    // nothing is drawn: no idle frames. Paper on a table does not hover.
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
    // Nothing moved since the last frame: draw nothing, and stop the loop.
    if (!busy && !first && !dirty) return false;
    dirty = false;
    const lie = lieOf(fold);
    aim(lie);
    crane.rotation.set(view.pitch - lie * Math.PI * 0.49, BASE_YAW * (1 - lie) + view.yaw, 0);
    mirror.rotation.copy(crane.rotation);
    renderer.render(scene, camera);
    if (first) {
      first = false;
      box.classList.add("on");
    }
    return busy || unfoldAt != null;
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
        dirty = true;
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
    dirty = true;
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
  const io = new IntersectionObserver(
    ([e]) => {
      onScreen = e.isIntersecting;
      if (e.intersectionRatio > 0.45 && fold === 0 && unfoldAt == null && !reduceMotion) unfoldAt = performance.now();
      wake();
    },
    { threshold: [0, 0.5] },
  );
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
    backing.geometry.dispose();
    backing.material.dispose();
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
    /** The fold: 0 flat, 1 folded. `immediate` follows the scroll position
     * directly; otherwise the crane eases there. */
    setFold(f, { immediate = false } = {}) {
      const g = Math.max(0, Math.min(1, f));
      if (g === goal && !immediate) return;
      goal = g;
      if (immediate && !(moment && !late && performance.now() - t0 < moment.fold + moment.foldFor * 2.2)) fold = g;
      dirty = true;
      wake();
    },
    dispose,
  };
}
