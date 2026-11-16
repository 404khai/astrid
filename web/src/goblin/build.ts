// Vendored from github.com/404khai/goblins (9e63cf1, MIT, Copyright (c) 2026 Khai; see web/src/goblin/LICENSE).
import * as THREE from "three";
import type { Goblin } from "./types";

export type MouthKind = "smile" | "grin" | "o" | "flat" | "frown" | "smirk";
export type PropKind = "magnifier" | "dots" | "question" | "zzz" | "hearts" | "sparkles" | "exclaim" | "sweat" | "tear";

export interface EyeRig {
  side: number;
  g: THREE.Group;
  open: THREE.Group;
  white: THREE.Object3D;
  pupil?: THREE.Object3D;
  look: { obj: THREE.Object3D; base: THREE.Vector3; range: number };
  lid?: THREE.Group;
  happy: THREE.Object3D;
  closed: THREE.Object3D;
}

export interface Rig {
  root: THREE.Group;
  head: THREE.Group;
  eyes: EyeRig[];
  brows: { obj: THREE.Object3D; base: THREE.Vector3; side: number }[];
  mouth: THREE.Group;
  mouths: Record<MouthKind, THREE.Object3D>;
  hands: THREE.Object3D[];
  handHome: THREE.Vector3[];
  props: Record<PropKind, THREE.Group>;
  ring: THREE.Mesh;
  top: number;
  state: { expr: string; lid: number; tilt: number; pupil: number; brow: number[]; nextBlink: number; blinkUntil: number };
}

// ---------- materials ----------
const ramp = new THREE.DataTexture(new Uint8Array([105, 105, 105, 255, 185, 185, 185, 255, 255, 255, 255, 255]), 3, 1);
ramp.minFilter = ramp.magFilter = THREE.NearestFilter;
ramp.needsUpdate = true;
const toonCache = new Map<string, THREE.Material>();
const flatCache = new Map<string, THREE.Material>();
const M = (c: string) => toonCache.get(c) ?? (toonCache.set(c, new THREE.MeshToonMaterial({ color: c, gradientMap: ramp })), toonCache.get(c)!);
const F = (c: string) => flatCache.get(c) ?? (flatCache.set(c, new THREE.MeshBasicMaterial({ color: c, side: THREE.DoubleSide })), flatCache.get(c)!);
const OUTLINE = new THREE.MeshBasicMaterial({ color: 0x1d2318, side: THREE.BackSide });
const INK = "#16140F";
const shade = (hex: string, f: number) => "#" + new THREE.Color(hex).multiplyScalar(f).getHexString();

type V3 = [number, number, number];
function mesh(geo: THREE.BufferGeometry, mat: THREE.Material, parent: THREE.Object3D, pos?: V3 | null, scl?: V3 | null, rot?: V3 | null, outline = true) {
  const m = new THREE.Mesh(geo, mat);
  if (pos) m.position.set(...pos);
  if (scl) m.scale.set(...scl);
  if (rot) m.rotation.set(...rot);
  if (outline) {
    const o = new THREE.Mesh(geo, OUTLINE);
    o.scale.setScalar(1.06);
    m.add(o);
  }
  parent.add(m);
  return m;
}
const S = (r: number, w = 28, h = 20) => new THREE.SphereGeometry(r, w, h);
const arc = (r: number, tube: number, a = Math.PI) => new THREE.TorusGeometry(r, tube, 10, 28, a);
const group = (parent: THREE.Object3D, pos?: V3, rot?: V3) => {
  const g = new THREE.Group();
  if (pos) g.position.set(...pos);
  if (rot) g.rotation.set(...rot);
  parent.add(g);
  return g;
};

function bentCone(r: number, h: number, bx: number, bz: number) {
  const g = new THREE.ConeGeometry(r, h, 32, 16);
  g.translate(0, h / 2, 0);
  const p = g.attributes.position;
  for (let i = 0; i < p.count; i++) {
    const t = p.getY(i) / h;
    p.setX(i, p.getX(i) + bx * t * t * t);
    p.setZ(i, p.getZ(i) + bz * t * t * t);
  }
  g.computeVertexNormals();
  return g;
}

export const LOW_HATS = ["gnome", "wizard", "beanie"];
const HEADS: Record<Goblin["head"], V3> = { round: [1.04, 0.96, 0.96], egg: [0.9, 1.1, 0.9], wide: [1.2, 0.86, 0.95], bean: [0.98, 1.0, 0.86] };

function heart(parent: THREE.Object3D, mat: THREE.Material, s: number) {
  const g = group(parent);
  g.scale.setScalar(s);
  for (const x of [-1, 1]) mesh(S(0.1, 16, 12), mat, g, [x * 0.075, 0, 0], [1, 1, 0.6]);
  mesh(new THREE.ConeGeometry(0.15, 0.2, 16), mat, g, [0, -0.1, 0], [1, 1, 0.45], [0, 0, Math.PI]);
  return g;
}
function drop(parent: THREE.Object3D, mat: THREE.Material, pos: V3, s = 1) {
  const g = group(parent, pos);
  g.scale.setScalar(s);
  mesh(S(0.08, 16, 12), mat, g, [0, 0, 0], [1, 1, 0.6]);
  mesh(new THREE.ConeGeometry(0.075, 0.13, 16), mat, g, [0, 0.09, 0], [1, 1, 0.6], null, false);
  return g;
}
function zee(parent: THREE.Object3D, mat: THREE.Material, s: number) {
  const g = group(parent);
  g.scale.setScalar(s);
  mesh(new THREE.BoxGeometry(0.3, 0.07, 0.07), mat, g, [0, 0.15, 0]);
  mesh(new THREE.BoxGeometry(0.3, 0.07, 0.07), mat, g, [0, -0.15, 0]);
  mesh(new THREE.BoxGeometry(0.4, 0.07, 0.07), mat, g, [0, 0, 0], null, [0, 0, Math.PI / 4 + 0.05]);
  return g;
}
function sparkle(parent: THREE.Object3D, mat: THREE.Material, s: number) {
  const g = group(parent);
  g.scale.setScalar(s);
  const o = new THREE.OctahedronGeometry(0.12);
  mesh(o, mat, g, null, [0.45, 1.5, 0.35], null, false);
  mesh(o, mat, g, null, [1.5, 0.45, 0.35], null, false);
  return g;
}

export function buildGoblin(d: Goblin): Rig {
  const root = new THREE.Group();
  const head = group(root);
  const [hx, hy, hz] = HEADS[d.head];
  const surf = (x: number, y: number) => hz * Math.sqrt(Math.max(0, 1 - (x / hx) ** 2 - (y / hy) ** 2));
  const skin = M(d.skin);
  const dark = M(shade(d.skin, 0.8));
  const ink = F(INK);
  const gold = M("#E0A92B");

  mesh(S(1, 40, 32), skin, head, [0, 0, 0], [hx, hy, hz]);

  // ---------- ears ----------
  const EARS = { pointy: [0.24, 0.9, -0.15], long: [0.2, 1.4, 0.35], droopy: [0.24, 0.95, -0.85], tiny: [0.17, 0.5, 0.05] } as const;
  for (const s of [-1, 1]) {
    if (d.ears === "round") {
      const ear = mesh(S(0.3), skin, head, [s * hx * 0.97, 0.12, -0.05], [1, 1, 0.38], [0, s * 1.1, 0]);
      mesh(S(0.18), dark, ear, [0, 0, 0.17], [1, 1, 0.4], null, false);
      continue;
    }
    const [r, h, a] = EARS[d.ears];
    const dir = new THREE.Vector2(s * Math.cos(a), Math.sin(a));
    const pos: V3 = [s * hx * 0.8 + dir.x * h * 0.45, 0.08 + dir.y * h * 0.45, -0.05];
    const ear = mesh(new THREE.ConeGeometry(r, h, 20), skin, head, pos, [1, 1, 0.45], [0, 0, Math.atan2(-dir.x, dir.y)]);
    mesh(new THREE.ConeGeometry(r * 0.55, h * 0.66, 16), dark, ear, [0, -h * 0.07, r * 0.32], [1, 1, 0.4], null, false);
  }

  // ---------- eyes ----------
  const EX = hx * 0.36, EY = 0.15;
  const EZ = surf(EX, EY) - 0.04;
  const white = M("#FFFFFF");
  const glow = F("#7CF2C8");
  const visor = d.eyes === "visor";
  if (visor) mesh(new THREE.BoxGeometry(hx * 1.2, 0.44, 0.3), F("#151515"), head, [0, EY, EZ + 0.02]);
  const eyes: EyeRig[] = [];
  for (const s of [-1, 1]) {
    const g = group(head, [s * EX, EY, visor ? EZ + 0.18 : EZ], [0, visor ? 0 : s * 0.32, 0]);
    const open = group(g);
    let w: THREE.Object3D, pupil: THREE.Object3D | undefined, look: EyeRig["look"], lid: THREE.Group | undefined;
    const wonkyBox = d.eyes === "wonky" && s > 0;
    const big = d.eyes === "wonky" && s < 0 ? 1.25 : 1;
    if (visor) {
      w = mesh(new THREE.BoxGeometry(0.24, 0.13, 0.05), glow, open, null, null, null, false);
      look = { obj: w, base: w.position.clone(), range: 0.07 };
    } else if (d.eyes === "beady") {
      w = mesh(S(0.12), F(INK), open, null, [1, 1.15, 0.5], null, false);
      look = { obj: w, base: w.position.clone(), range: 0.03 };
    } else {
      const shape: V3 = d.eyes === "oval" ? [0.85, 1.4, 0.55] : [1, 1.12, 0.55];
      w = wonkyBox
        ? mesh(new THREE.BoxGeometry(0.44, 0.32, 0.2), white, open, [0.04, -0.04, 0], null, [0, 0, -0.12])
        : mesh(S(0.22 * big), white, open, null, shape);
      pupil = mesh(S(0.125 * big, 20, 16), F(INK), open, [wonkyBox ? -0.02 : 0, wonkyBox ? -0.06 : 0, 0.115], [1, 1, 0.5], null, false);
      mesh(S(0.035 * big, 10, 8), F("#FFFFFF"), pupil, [0.045, 0.05, 0.11], null, null, false);
      look = { obj: pupil, base: pupil.position.clone(), range: 0.065 };
      // eyelid: a skin-colored dome hinged at the eye's center; rotation.x = 0 covers the top half
      if (!wonkyBox) {
        lid = group(open);
        const sc = (w as THREE.Mesh).scale;
        mesh(new THREE.SphereGeometry(0.235 * big, 24, 12, 0, Math.PI * 2, 0, Math.PI / 2), skin, lid, null, [sc.x * 1.04, sc.y * 1.04, sc.z * 1.15], null, false);
      } else {
        lid = group(open);
        mesh(new THREE.BoxGeometry(0.47, 0.18, 0.22), skin, lid, [0.04, 0.09, 0], null, [0, 0, -0.12], false);
      }
    }
    const arcMat = visor ? glow : F(INK);
    const happy = mesh(arc(0.13, 0.04), arcMat, g, [0, -0.05, visor ? 0.03 : 0.1], null, null, false);
    const closed = mesh(arc(0.13, 0.04), arcMat, g, [0, 0.05, visor ? 0.03 : 0.1], null, [0, 0, Math.PI], false);
    happy.visible = closed.visible = false;
    eyes.push({ side: s, g, open, white: w, pupil, look, lid, happy, closed });
  }

  // ---------- brows ----------
  const browMat = d.beard !== "none" ? M(shade(d.beardColor, 0.9)) : M(shade(d.skin, 0.55));
  const brows = [-1, 1].map((s) => {
    const base = new THREE.Vector3(s * EX, EY + (d.eyes === "oval" ? 0.42 : 0.36), surf(EX, EY + 0.36) + 0.04);
    const obj = mesh(S(0.15, 16, 10), browMat, head, [base.x, base.y, base.z], [1.25, 0.32, 0.4], [0, s * 0.3, 0]);
    // a low hat brim sits where the brows would be; the lids carry the mood instead (as in Classic)
    obj.visible = !LOW_HATS.includes(d.hat);
    return { obj, base, side: s };
  });

  // ---------- nose ----------
  const NY = -0.08, NZ = surf(0, NY);
  if (d.nose === "bulb") mesh(S(0.23), dark, head, [0, NY, NZ], [1, 0.9, 1]);
  else if (d.nose === "button") mesh(S(0.13), dark, head, [0, NY, NZ + 0.02], [1.15, 0.9, 1]);
  else if (d.nose === "long") mesh(new THREE.ConeGeometry(0.13, 0.7, 20), dark, head, [0, NY, NZ + 0.28], null, [Math.PI / 2 - 0.15, 0, 0]);
  else {
    mesh(S(0.16), dark, head, [0, NY + 0.04, NZ + 0.04]);
    mesh(new THREE.ConeGeometry(0.13, 0.42, 20), dark, head, [0, NY - 0.1, NZ + 0.2], null, [Math.PI / 2 + 0.7, 0, 0]);
  }

  // ---------- beard ----------
  const bc = M(d.beardColor);
  const hasBeard = d.beard !== "none";
  if (hasBeard) for (const s of [-1, 1]) mesh(S(0.2), bc, head, [s * 0.2, -0.3, surf(0.2, -0.3) + 0.04], [1.5, 0.6, 0.65], [0, 0, s * 0.35]);
  if (d.beard === "full") {
    mesh(S(0.9), bc, head, [0, -0.62, 0.32], [1.05 * hx, 1.05, 0.72]);
    mesh(S(0.5), bc, head, [0, -1.25, 0.42], [1, 1, 0.7]);
  } else if (d.beard === "pointy") {
    mesh(new THREE.ConeGeometry(0.72, 1.7, 28), bc, head, [0, -1.05, 0.42], [hx, 1, 0.62], [Math.PI + 0.12, 0, 0]);
  } else if (d.beard === "forked") {
    mesh(S(0.7), bc, head, [0, -0.6, 0.38], [1.1 * hx, 0.8, 0.7]);
    for (const s of [-1, 1]) mesh(new THREE.ConeGeometry(0.32, 1.1, 20), bc, head, [s * 0.28, -1.15, 0.48], [1, 1, 0.7], [Math.PI + 0.15, 0, -s * 0.25]);
  } else if (d.beard === "braid") {
    mesh(S(0.7), bc, head, [0, -0.55, 0.4], [1.1 * hx, 0.75, 0.7]);
    [0.3, 0.25, 0.21, 0.17].forEach((r, i) => mesh(S(r), bc, head, [0, -1.0 - i * 0.32, 0.55], [1, 0.9, 1]));
    mesh(S(0.1), M("#D8402F"), head, [0, -2.08, 0.55]);
  }

  // ---------- mouth ----------
  const MY = hasBeard ? -0.52 : -0.44;
  const mouth = group(head, [0, MY, hasBeard ? 1.0 : surf(0, MY) + 0.01]);
  const tongue = F("#E0566A");
  const grin = group(mouth);
  mesh(new THREE.CircleGeometry(0.17, 24, Math.PI, Math.PI), ink, grin, null, null, null, false);
  mesh(new THREE.CircleGeometry(0.075, 16), tongue, grin, [0, -0.1, 0.005], [1, 0.6, 1], null, false);
  const mouths: Record<MouthKind, THREE.Object3D> = {
    smile: mesh(arc(0.13, 0.035), ink, mouth, [0, 0.06, 0], null, [0, 0, Math.PI], false),
    grin,
    o: mesh(S(0.085, 16, 12), ink, mouth, null, [1, 1.25, 0.35], null, false),
    flat: mesh(new THREE.BoxGeometry(0.24, 0.045, 0.03), ink, mouth, null, null, null, false),
    frown: mesh(arc(0.12, 0.035), ink, mouth, [0, -0.06, 0], null, null, false),
    smirk: mesh(arc(0.1, 0.035, Math.PI * 0.8), ink, mouth, [0.04, 0.05, 0], null, [0, 0, Math.PI + 0.3], false),
  };
  Object.values(mouths).forEach((m) => (m.visible = false));
  if (d.extras.includes("tusks")) for (const s of [-1, 1]) mesh(new THREE.ConeGeometry(0.06, 0.2, 12), M("#F4EFE2"), mouth, [s * 0.17, 0.02, 0.02], null, [0, 0, -s * 0.2]);

  // ---------- hats ----------
  const hc = M(d.hatColor);
  const hcDark = M(shade(d.hatColor, 0.78));
  const capY = 0.48 * hy;
  let top = hy;
  if (d.hat === "gnome" || d.hat === "wizard") {
    const wiz = d.hat === "wizard";
    const h = wiz ? 2.3 : 2.0;
    const tipX = wiz ? -0.6 : 0.45, tipZ = wiz ? -0.4 : -0.55;
    const cone = mesh(bentCone(hx * 1.04, h, tipX, tipZ), hc, head, [0, capY, -0.05], [1, 1, hz / hx], [-0.12, 0, 0.06]);
    if (wiz) {
      mesh(new THREE.CylinderGeometry(hx * 1.65, hx * 1.65, 0.07, 48), hc, head, [0, capY + 0.02, -0.05], [1, 1, hz / hx], [-0.12, 0, 0.06]);
      mesh(new THREE.OctahedronGeometry(0.2), gold, cone, [tipX, h + 0.05, tipZ], [1, 1, 0.5]);
      mesh(new THREE.CylinderGeometry(hx * 1.05, hx * 1.05, 0.16, 40, 1, true), gold, cone, [0, 0.12, 0], null, null, false);
    } else {
      mesh(new THREE.TorusGeometry(hx * 0.99, 0.11, 12, 48), hcDark, head, [0, capY + 0.04, -0.05], [1, hz / hx, 1], [Math.PI / 2 - 0.12, 0, 0.06]);
    }
    top = capY + h * 0.8;
  } else if (d.hat === "beret") {
    mesh(S(1.08), hc, head, [-0.08, hy * 0.7, -0.05], [hx * 1.14, 0.36, 1.14], [0.1, 0, 0.22]);
    mesh(new THREE.CylinderGeometry(0.04, 0.06, 0.2, 10), hc, head, [-0.02, hy * 0.7 + 0.4, -0.05], null, [0, 0, 0.2]);
    top = hy + 0.35;
  } else if (d.hat === "beanie") {
    mesh(new THREE.SphereGeometry(1.06, 32, 16, 0, Math.PI * 2, 0, Math.PI / 2), hc, head, [0, capY - 0.15, 0], [hx * 1.02, hy, hz * 1.02]);
    mesh(new THREE.CylinderGeometry(hx * 1.1, hx * 1.1, 0.3, 40), hcDark, head, [0, capY - 0.08, 0], [1, 1, hz / hx]);
    mesh(S(0.26), M("#ECE6D6"), head, [0, capY + hy * 0.95, 0]);
    top = capY + hy + 0.2;
  } else if (d.hat === "mushroom") {
    const cap = mesh(new THREE.SphereGeometry(1.5, 40, 20, 0, Math.PI * 2, 0, Math.PI / 2), hc, head, [0, capY + 0.05, 0], [hx * 1.02, 0.75, 1]);
    mesh(new THREE.CircleGeometry(1.5, 40), M("#F2ECDD"), cap, [0, 0.001, 0], null, [Math.PI / 2, 0, 0], false);
    const spots: V3[] = [[0, 1.0, 0.55], [0.7, 0.7, 0.75], [-0.75, 0.75, 0.6], [0.35, 1.1, -0.4], [-0.5, 0.95, -0.6], [1.15, 0.5, -0.3], [-1.1, 0.45, 0.4]];
    for (const p of spots) {
      const v = new THREE.Vector3(...p).normalize().multiplyScalar(1.48);
      const spot = mesh(S(0.2, 16, 10), M("#F7F2E6"), cap, [v.x, v.y, v.z], [1, 1, 0.25], null, false);
      spot.lookAt(v.clone().multiplyScalar(2));
    }
    top = capY + 1.1;
  } else if (d.hat === "sprout") {
    const leaves: [number, number, number][] = [[0, 0.5, 0], [-0.25, 0.4, 0.5], [0.25, 0.4, -0.5]];
    for (const [x, h, rz] of leaves) mesh(S(0.2), hc, head, [x, hy + h * 0.6, 0], [0.75, h * 3.2, 0.35], [0, 0, rz]);
    top = hy + 0.6;
  } else if (d.hat === "headband") {
    mesh(new THREE.TorusGeometry(hx * 0.97, 0.09, 12, 48), hc, head, [0, capY, 0], [1, hz / hx, 1], [Math.PI / 2 - 0.1, 0, 0]);
    for (const s of [-1, 1]) mesh(new THREE.BoxGeometry(0.12, 0.5, 0.05), hc, head, [s * 0.15, capY - 0.25, -hz * 0.85], null, [0.2, 0, s * 0.4]);
  }

  // ---------- scarf ----------
  if (d.scarf !== "none") {
    const sc = M(d.scarfColor);
    mesh(new THREE.TorusGeometry(0.72, 0.24, 16, 40), sc, head, [0, -0.86 * hy, 0], [hx, 1, 0.9], [Math.PI / 2 - 0.1, 0, 0]);
    if (d.scarf === "long") {
      mesh(new THREE.BoxGeometry(0.38, 1.1, 0.14), sc, head, [0.62, -1.35, 0.48], null, [0.1, 0, 0.18]);
      mesh(new THREE.BoxGeometry(0.38, 0.12, 0.15), M(shade(d.scarfColor, 0.75)), head, [0.72, -1.82, 0.53], null, [0.1, 0, 0.18], false);
    } else mesh(S(0.2), sc, head, [0.45, -0.95, 0.62], [1.1, 0.9, 0.8]);
  }

  // ---------- eyewear ----------
  if (d.eyewear === "round" || d.eyewear === "shades") {
    for (const s of [-1, 1]) {
      const g = group(head, [s * EX, EY, EZ + 0.2], [0, s * 0.32, 0]);
      if (d.eyewear === "round") mesh(new THREE.TorusGeometry(0.25, 0.035, 10, 32), ink, g, null, null, null, false);
      else mesh(new THREE.CylinderGeometry(0.27, 0.27, 0.05, 32), ink, g, null, null, [Math.PI / 2, 0, 0], false);
      mesh(new THREE.BoxGeometry(0.04, 0.04, 0.9), ink, head, [s * hx * 0.94, EY + 0.02, 0.48], null, [0, s * 0.35, 0], false);
    }
    mesh(arc(0.1, 0.03), ink, head, [0, EY, EZ + 0.24], null, null, false);
  } else if (d.eyewear === "monocle") {
    const g = group(head, [EX, EY, EZ + 0.2], [0, 0.32, 0]);
    mesh(new THREE.TorusGeometry(0.27, 0.04, 10, 32), gold, g, null, null, null, false);
    for (let i = 0; i < 6; i++) mesh(new THREE.TorusGeometry(0.035, 0.012, 6, 12), gold, head, [EX + 0.25 + i * 0.03, EY - 0.27 - i * 0.13, EZ + 0.12 - i * 0.04], null, [0, i % 2 ? Math.PI / 2 : 0, 0], false);
  }

  // ---------- extras ----------
  if (d.extras.includes("headphones")) {
    const band = F("#25221F"), cup = M("#ECE6D6");
    const R = d.hat === "gnome" || d.hat === "wizard" || d.hat === "mushroom" ? hx * 1.3 : hx * 1.17;
    mesh(new THREE.TorusGeometry(R, 0.07, 10, 40, Math.PI), band, head, [0, 0.12, 0.05], [1, hy / hx + 0.05, 1], null, false);
    for (const s of [-1, 1]) {
      const c = mesh(new THREE.CylinderGeometry(0.34, 0.34, 0.24, 28), cup, head, [s * hx * 1.04, 0.1, 0.05], null, [0, 0, Math.PI / 2]);
      mesh(new THREE.CylinderGeometry(0.22, 0.22, 0.26, 24), band, c, null, null, null, false);
    }
  }
  if (d.extras.includes("earring")) mesh(new THREE.TorusGeometry(0.1, 0.03, 8, 20), gold, head, [-hx * 1.06, -0.1, 0], null, [0, Math.PI / 2, 0]);
  if (d.extras.includes("freckles")) for (const s of [-1, 1]) for (const [x, y] of [[0.5, -0.12], [0.62, -0.2], [0.48, -0.26]]) {
    const fx = s * x * hx;
    mesh(S(0.035, 8, 6), dark, head, [fx, y, surf(fx, y) + 0.005], [1, 1, 0.3], null, false);
  }
  if (d.extras.includes("pipe")) {
    const wood = M("#6B4226");
    const pz = mouth.position.z;
    mesh(new THREE.CylinderGeometry(0.03, 0.04, 0.6, 10), F("#25221F"), head, [0.28, MY + 0.02, pz + 0.12], null, [0.3, 0, 1.2], false);
    const bowl = mesh(new THREE.CylinderGeometry(0.13, 0.1, 0.28, 16), wood, head, [0.58, MY + 0.08, pz + 0.2]);
    mesh(new THREE.CylinderGeometry(0.1, 0.1, 0.02, 16), F("#25221F"), bowl, [0, 0.14, 0], null, null, false);
  }

  // ---------- hands (Mipmapp-style floating mitts) ----------
  const handHome = [-1, 1].map((s) => new THREE.Vector3(s * (hx + 0.2), -1.2, 0.35));
  const hands = handHome.map((p) => mesh(S(0.21), skin, root, [p.x, p.y, p.z], [1, 0.95, 0.85]));

  // ---------- props ----------
  const yellow = M("#F2B630"), red = M("#E2443A"), blue = M("#7EC8F0"), paper = M("#F7F4EC");
  const props = {} as Record<PropKind, THREE.Group>;
  const P = (k: PropKind, parent: THREE.Object3D, pos: V3 = [0, 0, 0]) => {
    const g = group(parent, pos);
    g.visible = false;
    props[k] = g;
    return g;
  };
  // magnifier sits in the right mitt
  const mag = P("magnifier", hands[1], [0, 0.1, 0.1]);
  mesh(new THREE.CylinderGeometry(0.045, 0.05, 0.45, 12), M("#6B4226"), mag, [0, 0.12, 0]);
  const lens = group(mag, [0, 0.6, 0]);
  mesh(new THREE.TorusGeometry(0.26, 0.05, 12, 32), M("#3A3A44"), lens);
  mesh(new THREE.CircleGeometry(0.25, 32), new THREE.MeshBasicMaterial({ color: "#CFF1FF", transparent: true, opacity: 0.35, side: THREE.DoubleSide }), lens, null, null, null, false);
  const dots = P("dots", root, [hx + 0.35, hy + 0.35, 0.2]);
  [0.08, 0.12, 0.17].forEach((r, i) => mesh(S(r, 16, 12), paper, dots, [i * 0.25, i * 0.22, 0]));
  const q = P("question", root, [hx * 0.9, top + 0.15, 0.2]);
  mesh(arc(0.2, 0.07, Math.PI * 1.5), yellow, q, [0, 0.25, 0], null, [0, 0, -Math.PI / 2]);
  mesh(new THREE.CylinderGeometry(0.07, 0.07, 0.18, 12), yellow, q, [0, -0.02, 0]);
  mesh(S(0.08, 12, 10), yellow, q, [0, -0.28, 0]);
  const zz = P("zzz", root, [hx * 0.7, hy * 0.9, 0.2]);
  [0.7, 0.9, 1.1].forEach((s) => zee(zz, F(INK), s));
  const hearts = P("hearts", root, [0, hy, 0.4]);
  [1, 0.8, 0.65].forEach((s) => heart(hearts, red, s));
  const sp = P("sparkles", root, [0, 0.2, 0]);
  for (let i = 0; i < 5; i++) sparkle(sp, yellow, 0.8 + (i % 2) * 0.4);
  const ex = P("exclaim", root, [-hx - 0.35, hy + 0.3, 0.2]);
  mesh(new THREE.BoxGeometry(0.13, 0.45, 0.1), red, ex, [0, 0.12, 0], null, [0, 0, -0.15]);
  mesh(S(0.08, 12, 10), red, ex, [-0.05, -0.25, 0]);
  const sweat = P("sweat", head, [hx * 0.8, 0.45, 0.5]);
  drop(sweat, blue, [0, 0, 0], 1.3);
  const tear = P("tear", head, [-EX - 0.05, EY - 0.22, EZ + 0.12]);
  drop(tear, blue, [0, 0, 0]);

  // ---------- ground ----------
  const shadow = new THREE.Mesh(new THREE.CircleGeometry(1.1, 40), new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.12 }));
  shadow.rotation.x = -Math.PI / 2;
  shadow.position.y = -2.25;
  root.add(shadow);
  const ring = new THREE.Mesh(new THREE.RingGeometry(1.15, 1.32, 48), new THREE.MeshBasicMaterial({ color: 0xd8402f }));
  ring.rotation.x = -Math.PI / 2;
  ring.position.y = -2.24;
  root.add(ring);

  return {
    root, head, eyes, brows, mouth, mouths, hands, handHome, props, ring, top,
    state: { expr: "", lid: 0, tilt: 0, pupil: 1, brow: [0, 0, 0, 0], nextBlink: 1 + Math.random() * 3, blinkUntil: 0 },
  };
}

export function disposeRig(r: Rig) {
  r.root.traverse((o) => (o as THREE.Mesh).geometry?.dispose());
}

/** Shared toon kit so scenery matches the goblins. */
export const kit = { toon: M, flat: F, mesh, sphere: S, shade, OUTLINE };
