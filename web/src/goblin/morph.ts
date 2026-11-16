// Vendored from github.com/404khai/goblins (9e63cf1, MIT, Copyright (c) 2026 Khai; see web/src/goblin/LICENSE).
import * as THREE from "three";
import { kit, type Rig } from "./build";
import type { Goblin } from "./types";

/**
 * Spells: a goblin dissolves into a swarm of its own colors, swirls over a rune circle, and
 * reforms as a shape that shows what it is doing (writing, sending, powering down…).
 * Particles are toon-shaded instances, so the look stays flat and outlined like the goblins.
 */

export type MorphId =
  | "writing" | "sketching" | "solving" | "listening" | "thinking" | "orbit" | "radar" | "progress" | "dictating"
  | "sending" | "receiving" | "uploading" | "bouncing" | "alerting" | "powering-down";

interface Particle {
  k: number; // 0..1, this particle's slot in the target shape
  r: [number, number, number]; // stable randoms
  out: THREE.Vector3;
  s: number; // scale
  c: string | null; // target color, null keeps the goblin's own
}
interface Palette { skin: string; hat: string; beard: string; scarf: string; accent: string }
interface MorphDef { label: string; blurb: string; accent: string; shape: (p: Particle, t: number, g: Palette) => void }

const INK = "#1D2318", PAPER = "#F4EEDC", GOLD = "#F2B630", TEAL = "#7CF2C8", RED = "#E2443A";
const TAU = Math.PI * 2;
const { sin, cos, abs, floor, min, max, sqrt } = Math;
const frac = (x: number) => x - floor(x);
const smooth = (x: number) => { x = min(1, max(0, x)); return x * x * (3 - 2 * x); };
const shadeHex = (hex: string) => "#" + new THREE.Color(hex).multiplyScalar(0.75).getHexString();
const sphere = (p: Particle, cx: number, cy: number, cz: number, r: number, sz = 1) => {
  const a = p.r[1] * TAU, z = p.r[2] * 2 - 1, q = sqrt(1 - z * z);
  p.out.set(cx + q * cos(a) * r, cy + z * r, cz + q * sin(a) * r * sz);
};

export const MORPHS: Record<MorphId, MorphDef> = {
  writing: {
    label: "Writing", blurb: "Its hat becomes a quill and writes onto a scroll.", accent: GOLD,
    shape: (p, t, g) => {
      const prog = frac(t / 7);
      const nibLine = min(3, floor(prog * 4)), nibU = frac(prog * 4);
      const nib = [-1.25 + 2.5 * nibU, -0.55 - nibLine * 0.34 + 0.06 * sin(nibU * 40 + nibLine), 0.5];
      if (p.k < 0.5) {
        // parchment with rolled ends
        const u = p.r[0], w = p.r[1];
        let x = -1.55 + 3.1 * u, z = 0.3 + 0.1 * sin(u * TAU);
        if (u < 0.08 || u > 0.92) {
          const a = p.r[2] * TAU;
          x = (u < 0.5 ? -1.6 : 1.6) + 0.13 * cos(a);
          z = 0.32 + 0.13 * sin(a);
        }
        p.out.set(x, -1.95 + 1.7 * w, z);
        p.c = u < 0.08 || u > 0.92 ? "#D9C9A0" : PAPER;
      } else if (p.k < 0.7) {
        const j = (p.k - 0.5) / 0.2, line = floor(j * 4), u = frac(j * 4);
        if ((line + u) / 4 < prog) {
          p.out.set(-1.25 + 2.5 * u, -0.55 - line * 0.34 + 0.06 * sin(u * 40 + line), 0.42);
          p.s = 0.75;
        } else { p.out.set(nib[0], nib[1], nib[2]); p.s = 0; }
        p.c = INK;
      } else {
        // the quill: a curved feather whose nib follows the writing
        const a = (p.k - 0.7) / 0.3;
        const wob = sin(t * 18) * 0.04;
        const sx = nib[0] + 0.25 * a + 0.35 * a * a + wob, sy = nib[1] + 2.1 * a, sz = nib[2] + 0.2 * a;
        const width = a < 0.15 ? 0.03 : sin(Math.PI * ((a - 0.15) / 0.85) ** 0.7) * 0.42;
        const side = (p.r[1] - 0.5) * 2;
        p.out.set(sx + side * width * 0.85, sy - abs(side) * width * 0.35, sz + side * width * 0.2);
        p.c = a < 0.15 ? INK : side > 0.85 || side < -0.85 ? g.beard : g.hat;
      }
    },
  },
  sketching: {
    label: "Sketching", blurb: "A pencil draws a star on a little easel.", accent: GOLD,
    shape: (p, t, g) => {
      const prog = min(1, frac(t / 6) * 1.2);
      const curve = (u: number) => {
        const a = u * TAU + Math.PI / 2, r = 0.62 + 0.3 * cos(5 * (a - Math.PI / 2));
        return [cos(a) * r, 0.45 + sin(a) * r, 0.42];
      };
      if (p.k < 0.45) {
        const legs = p.r[0] < 0.12;
        if (legs) { const side = p.r[1] < 0.5 ? -1 : 1; const y = p.r[2]; p.out.set(side * (0.9 + y * 0.25), -0.9 - y * 1.3, 0.1); p.c = "#7A5236"; }
        else { p.out.set((p.r[1] - 0.5) * 2.6, -0.9 + p.r[2] * 2.6, 0.3); p.c = PAPER; }
      } else if (p.k < 0.72) {
        const u = (p.k - 0.45) / 0.27;
        const [x, y, z] = curve(u <= prog ? u : prog);
        p.out.set(x, y, z);
        p.s = u <= prog ? 0.8 : 0;
        p.c = INK;
      } else {
        // pencil, tip on the line being drawn
        const [nx, ny, nz] = curve(prog);
        const a = (p.k - 0.72) / 0.28, ang = 0.75 + sin(t * 9) * 0.05;
        const along = a * 1.9, rad = a < 0.12 ? a / 0.12 * 0.14 : 0.14;
        const ra = p.r[1] * TAU;
        p.out.set(nx + cos(ang) * along - sin(ang) * cos(ra) * rad, ny + sin(ang) * along + cos(ang) * cos(ra) * rad, nz + 0.25 + sin(ra) * rad);
        p.c = a < 0.06 ? INK : a < 0.14 ? "#E9C9A0" : a > 0.9 ? "#E78FA8" : a > 0.85 ? "#C9C2B0" : g.hat;
      }
    },
  },
  solving: {
    label: "Solving", blurb: "Becomes a puzzle cube that twists itself and solves it.", accent: TEAL,
    shape: (p, t, g) => {
      const S = 0.82, face = floor(p.r[0] * 6);
      const u = (p.r[1] * 2 - 1) * S, w = (p.r[2] * 2 - 1) * S;
      const v = [
        new THREE.Vector3(S, u, w), new THREE.Vector3(-S, u, w), new THREE.Vector3(u, S, w),
        new THREE.Vector3(u, -S, w), new THREE.Vector3(u, w, S), new THREE.Vector3(u, w, -S),
      ][face];
      const grid = (c: number) => { const f = frac(((c / S) + 1) * 1.5); return f < 0.09 || f > 0.91; };
      p.c = grid(u) || grid(w) ? INK : [g.hat, g.beard, g.skin, GOLD, TEAL, RED][face];
      // twist a layer a quarter turn and back: scramble, then solve
      const step = floor(t / 1.6), q = frac(t / 1.6);
      const ang = (smooth(q / 0.4) - smooth((q - 0.5) / 0.4)) * (Math.PI / 2) * (step % 2 ? 1 : -1);
      const axis = ["y", "x", "z"][step % 3] as "x" | "y" | "z";
      const layer = (step * 7) % 3 - 1;
      const c = v[axis];
      if ((c < -S / 3 ? -1 : c > S / 3 ? 1 : 0) === layer) v.applyAxisAngle(new THREE.Vector3(axis === "x" ? 1 : 0, axis === "y" ? 1 : 0, axis === "z" ? 1 : 0), ang);
      v.applyEuler(new THREE.Euler(0.5, t * 0.4 + 0.6, 0));
      p.out.copy(v).add(new THREE.Vector3(0, 0.2, 0));
    },
  },
  listening: {
    label: "Listening", blurb: "One big ear, with sound waves rolling in.", accent: TEAL,
    shape: (p, t, g) => {
      const cx = -0.35, cy = 0.2;
      if (p.k < 0.5) {
        const a = -1.9 + p.r[0] * 4.0, th = (p.r[1] - 0.5) * 0.36;
        const R = 1.05 + th;
        p.out.set(cx + cos(a) * R * 0.8, cy + sin(a) * R, (p.r[2] - 0.5) * 0.3);
        p.c = g.skin;
      } else if (p.k < 0.62) {
        const a = -1.2 + p.r[0] * 3.2, R = 0.42 + (p.r[1] - 0.5) * 0.14;
        p.out.set(cx + 0.1 + cos(a) * R * 0.8, cy + sin(a) * R, 0.15);
        p.c = shadeHex(g.skin);
      } else {
        const q = frac(p.r[0] + t * 0.45);
        const R = 2.9 - q * 1.6, a = (p.r[1] - 0.5) * 1.1;
        p.out.set(cx + cos(a) * R, cy + sin(a) * R, 0.2 + (p.r[2] - 0.5) * 0.1);
        p.s = sin(q * Math.PI) * 1.1;
        p.c = TEAL;
      }
    },
  },
  thinking: {
    label: "Thinking", blurb: "A cloud of thoughts with bubbles drifting up.", accent: TEAL,
    shape: (p, t) => {
      if (p.k < 0.78) {
        const B = [[-0.8, 1.15, 0.62], [0, 1.6, 0.75], [0.85, 1.2, 0.65], [-0.35, 0.75, 0.55], [0.5, 0.75, 0.55], [1.35, 0.8, 0.45]];
        const [x, y, r] = B[floor(p.r[0] * B.length)];
        sphere(p, x, y + sin(t * 1.5 + x) * 0.06, 0.1, r, 0.6);
        p.c = "#F7F4EC";
      } else {
        const j = floor(((p.k - 0.78) / 0.22) * 3);
        const pulse = 1 + 0.25 * sin(t * 4 - j * 1.2);
        const C = [[-0.9, -0.25, 0.3], [-1.25, -0.85, 0.22], [-1.45, -1.35, 0.14]][j];
        sphere(p, C[0], C[1], 0.2, C[2] * pulse);
        p.c = "#F7F4EC";
      }
    },
  },
  orbit: {
    label: "Orbit", blurb: "A little planet with three rings spinning around it.", accent: GOLD,
    shape: (p, t, g) => {
      if (p.k < 0.38) { sphere(p, 0, 0.2, 0, 0.78); p.c = null; return; }
      const ring = floor(p.r[0] * 3);
      const a = p.r[1] * TAU + t * (0.9 + ring * 0.45) * (ring % 2 ? -1 : 1);
      const R = 1.35 + ring * 0.38;
      const v = new THREE.Vector3(cos(a) * R, 0, sin(a) * R);
      v.applyEuler(new THREE.Euler(0.35 + ring * 0.5, ring * 1.1, 0.25 - ring * 0.35));
      p.out.copy(v).add(new THREE.Vector3(0, 0.2, 0));
      p.c = [g.hat, g.beard, GOLD][ring];
      p.s = p.r[2] < 0.03 ? 3 : 0.8;
    },
  },
  radar: {
    label: "Radar", blurb: "Pulses roll out while a beam sweeps around.", accent: TEAL,
    shape: (p, t) => {
      if (p.k < 0.7) {
        const rad = frac(p.r[0] + t * 0.45) * 2.1, a = p.r[1] * TAU;
        p.out.set(cos(a) * rad, 0.2 + sin(a) * rad, 0.2);
        p.s = 1.1 - rad / 2.2;
        p.c = TEAL;
      } else if (p.k < 0.95) {
        const a = -t * 2.2 - p.r[1] * 0.5, rad = 0.15 + p.r[0] * 1.95;
        p.out.set(cos(a) * rad, 0.2 + sin(a) * rad, 0.25);
        p.s = 1.2 - p.r[1];
        p.c = "#E9FFF7";
      } else { sphere(p, 0, 0.2, 0.25, 0.14); p.c = INK; }
    },
  },
  progress: {
    label: "Progress", blurb: "A ring that fills up, then starts over.", accent: GOLD,
    shape: (p, t, g) => {
      const prog = min(1, frac(t / 6) * 1.15);
      if (p.k < 0.82) {
        const u = p.k / 0.82, a = Math.PI / 2 - u * TAU;
        const th = (p.r[1] - 0.5) * 0.32, dz = (p.r[2] - 0.5) * 0.25;
        p.out.set(cos(a) * (1.5 + th), 0.2 + sin(a) * (1.5 + th), dz);
        const on = u < prog;
        p.c = on ? g.hat : "#B9B29C";
        p.s = on ? 1.15 : 0.55;
      } else { sphere(p, 0, 0.2, 0, 0.32 + 0.08 * sin(t * 5)); p.c = prog >= 1 ? GOLD : g.skin; }
    },
  },
  dictating: {
    label: "Speaking", blurb: "Its voice turns into dancing sound bars.", accent: RED,
    shape: (p, t, g) => {
      const bar = floor(p.r[0] * 9), x = -1.6 + bar * 0.4;
      const h = 0.25 + 1.55 * abs(sin(t * 5.5 + bar * 0.9) * sin(t * 2.1 + bar * 0.4));
      p.out.set(x + (p.r[2] - 0.5) * 0.2, 0.2 + (p.r[1] - 0.5) * h * 2, (p.r[2] - 0.5) * 0.2);
      p.c = bar % 2 ? g.hat : g.skin;
    },
  },
  sending: {
    label: "Sending", blurb: "Folds into a paper plane and loops off.", accent: TEAL,
    shape: (p, t) => {
      const path = (tt: number) => new THREE.Vector3(sin(tt * 1.2) * 1.5, 0.4 + sin(tt * 2.4) * 0.55, cos(tt * 1.2) * 0.7);
      if (p.k < 0.75) {
        const tri = floor(p.r[0] * 3);
        let a = p.r[1], b = p.r[2];
        if (a + b > 1) { a = 1 - a; b = 1 - b; }
        const N = new THREE.Vector3(1, 0, 0), BC = new THREE.Vector3(-0.8, 0, 0);
        const corner = [new THREE.Vector3(-0.8, 0.05, 0.9), new THREE.Vector3(-0.8, 0.05, -0.9), new THREE.Vector3(-0.8, -0.4, 0)][tri];
        const v = N.clone().multiplyScalar(1 - a - b).add(corner.clone().multiplyScalar(a)).add(BC.clone().multiplyScalar(b));
        const pos = path(t), ahead = path(t + 0.05).sub(pos);
        v.multiplyScalar(1.05).applyAxisAngle(new THREE.Vector3(0, 1, 0), Math.atan2(-ahead.z, ahead.x));
        v.applyAxisAngle(new THREE.Vector3(0, 0, 1), Math.atan2(ahead.y, Math.hypot(ahead.x, ahead.z)) * 0.6);
        p.out.copy(pos).add(v);
        p.c = tri === 2 ? "#D9D2BE" : PAPER;
      } else {
        const back = (p.k - 0.75) / 0.25;
        p.out.copy(path(t - 0.15 - back * 1.3)).add(new THREE.Vector3(0, (p.r[1] - 0.5) * 0.15, 0));
        p.s = (1 - back) * 0.9;
        p.c = TEAL;
      }
    },
  },
  receiving: {
    label: "Receiving", blurb: "An envelope gathers a stream of sparks.", accent: GOLD,
    shape: (p, t, g) => {
      const W = 1.3, H = 0.85, cy = 0.1;
      if (p.k < 0.45) {
        p.out.set((p.r[0] - 0.5) * 2 * W, cy + (p.r[1] - 0.5) * 2 * H, 0.2);
        p.c = PAPER;
      } else if (p.k < 0.7) {
        const u = p.r[0], side = p.r[1] < 0.5 ? -1 : 1;
        p.out.set(side * W * (1 - u), cy + H - u * H * 1.05, 0.28);
        p.c = g.hat;
      } else {
        const q = frac(p.r[0] + t * 0.6);
        const from = new THREE.Vector3(2.6 + p.r[1], 2.6 + p.r[2], 0.8);
        p.out.copy(from).lerp(new THREE.Vector3(0, cy, 0.3), q * q);
        p.s = 1 - q * 0.7;
        p.c = GOLD;
      }
    },
  },
  uploading: {
    label: "Uploading", blurb: "Becomes an arrow riding a stream of bits.", accent: TEAL,
    shape: (p, t, g) => {
      const lift = sin(t * 2) * 0.12;
      if (p.k < 0.4) { p.out.set((p.r[0] - 0.5) * 0.6, -1.5 + p.r[1] * 1.9 + lift, (p.r[2] - 0.5) * 0.3); p.c = g.hat; }
      else if (p.k < 0.7) {
        const y = p.r[1], half = (1 - y) * 0.95;
        p.out.set((p.r[0] - 0.5) * 2 * half, 0.4 + y * 1.3 + lift, (p.r[2] - 0.5) * 0.3);
        p.c = g.hat;
      } else {
        const q = frac(p.r[0] + t * 0.5);
        p.out.set((p.r[1] - 0.5) * 2.4, -2.2 + q * 4.6, (p.r[2] - 0.5) * 1.2);
        p.s = sin(q * Math.PI) * 0.8;
        p.c = TEAL;
      }
    },
  },
  bouncing: {
    label: "Bouncing", blurb: "Rolls itself into a ball and bounces.", accent: RED,
    shape: (p, t) => {
      const b = abs(sin(t * 3));
      const squash = 1 - 0.32 * (1 - b) ** 6;
      if (p.k < 0.85) {
        const a = p.r[1] * TAU, z = p.r[2] * 2 - 1, q = sqrt(1 - z * z);
        const r = 0.82;
        p.out.set(q * cos(a) * r / sqrt(squash), -1.35 + 2.6 * b + z * r * squash, q * sin(a) * r);
        p.c = null;
      } else {
        const a = p.r[1] * TAU, rr = sqrt(p.r[2]) * (0.5 + 0.6 * (1 - b));
        p.out.set(cos(a) * rr, -2.17, sin(a) * rr * 0.6);
        p.s = 0.6;
        p.c = INK;
      }
    },
  },
  alerting: {
    label: "Alerting", blurb: "A swinging bell that rings out sparks.", accent: RED,
    shape: (p, t) => {
      const swing = sin(t * 7) * 0.32;
      const pivot = new THREE.Vector3(0, 1.4, 0);
      if (p.k < 0.8) {
        const y = p.r[1], a = p.r[2] * TAU;
        const r = 0.28 + 0.95 * y ** 1.6 + (y > 0.93 ? 0.12 : 0);
        const v = new THREE.Vector3(cos(a) * r, 1.2 - y * 2.2, sin(a) * r);
        v.sub(pivot).applyAxisAngle(new THREE.Vector3(0, 0, 1), swing).add(pivot);
        p.out.copy(v);
        p.c = GOLD;
      } else if (p.k < 0.86) {
        sphere(p, 0, -1.25, 0, 0.22);
        p.out.sub(pivot).applyAxisAngle(new THREE.Vector3(0, 0, 1), swing * 1.6).add(pivot);
        p.c = INK;
      } else {
        const ray = floor(p.r[0] * 8), a = (ray / 8) * TAU + 0.2, q = frac(p.r[1] + t * 1.2);
        const rad = 1.7 + q * 1.1;
        p.out.set(cos(a) * rad, 0.1 + sin(a) * rad, 0.2);
        p.s = (1 - q) * 1.1;
        p.c = RED;
      }
    },
  },
  "powering-down": {
    label: "Powering down", blurb: "Spirals into a tiny ember that blinks off.", accent: "#9DA48E",
    shape: (p, t) => {
      const q = frac(t / 5);
      const rad = 2 * p.r[0] * Math.exp(-q * 3.5) + 0.12;
      const a = p.r[1] * TAU + t * (2.5 / (rad + 0.25));
      p.out.set(cos(a) * rad, 0.2 + (p.r[2] - 0.5) * rad * 0.5, sin(a) * rad * 0.6);
      p.s = max(0.25, 1 - q) * (q > 0.85 ? (sin(t * 30) > 0 ? 1 : 0.2) : 1);
      p.c = q > 0.6 ? "#5E6552" : null;
    },
  },
};
export const MORPH_IDS = Object.keys(MORPHS) as MorphId[];

const N = 640;

export class Morpher {
  private group = new THREE.Group();
  private bits: THREE.InstancedMesh;
  private stars: THREE.InstancedMesh;
  private circle = new THREE.Group();
  private src: THREE.Vector3[] = [];
  private srcColor: THREE.Color[] = [];
  private pos: THREE.Vector3[] = [];
  private ps: Particle[] = [];
  private palette: Palette;
  private id: MorphId | null = null;
  private shown: MorphId | null = null;
  private m = 0;
  private last = 0;
  private dummy = new THREE.Object3D();
  private col = new THREE.Color();
  private tmp = new THREE.Color();
  private runeMat: THREE.MeshBasicMaterial;

  constructor(private rig: Rig, goblin: Goblin) {
    this.palette = { skin: goblin.skin, hat: goblin.hat === "none" ? goblin.skin : goblin.hatColor, beard: goblin.beard === "none" ? goblin.skin : goblin.beardColor, scarf: goblin.scarfColor, accent: GOLD };
    rig.root.add(this.group);

    // sample the goblin's own surface, in root space, with its colors
    rig.root.updateMatrixWorld(true);
    const inv = rig.root.matrixWorld.clone().invert();
    const pool: { v: THREE.Vector3; c: THREE.Color }[] = [];
    rig.head.traverse((o) => {
      const mesh = o as THREE.Mesh;
      if (!mesh.isMesh || mesh.material === kit.OUTLINE || !o.visible) return;
      const mat = mesh.material as THREE.MeshBasicMaterial;
      if (!mat.color || mat.transparent) return;
      const pa = mesh.geometry.attributes.position;
      const step = max(1, floor(pa.count / 220));
      for (let i = 0; i < pa.count; i += step) pool.push({ v: new THREE.Vector3().fromBufferAttribute(pa, i).applyMatrix4(mesh.matrixWorld).applyMatrix4(inv), c: mat.color });
    });
    let seed = 11;
    const rnd = () => ((seed = (seed * 16807) % 2147483647) - 1) / 2147483646;
    const picks = Array.from({ length: N }, () => pool[floor(rnd() * pool.length)]);
    picks.sort((a, b) => a.v.y - b.v.y); // low parts fill the first slots of each shape
    picks.forEach((s, i) => {
      this.src.push(s.v.clone());
      this.srcColor.push(s.c.clone());
      this.pos.push(s.v.clone());
      this.ps.push({ k: i / N, r: [rnd(), rnd(), rnd()], out: new THREE.Vector3(), s: 1, c: null });
    });

    const base = kit.toon("#ffffff") as THREE.MeshToonMaterial;
    this.bits = new THREE.InstancedMesh(new THREE.IcosahedronGeometry(0.075, 0), new THREE.MeshToonMaterial({ color: "#ffffff", gradientMap: base.gradientMap }), N);
    this.bits.frustumCulled = false;
    this.group.add(this.bits);
    const star = new THREE.OctahedronGeometry(0.09);
    star.scale(0.5, 1.6, 0.4);
    this.stars = new THREE.InstancedMesh(star, new THREE.MeshBasicMaterial({ color: GOLD }), 48);
    this.stars.frustumCulled = false;
    this.group.add(this.stars);

    // rune circle under the feet
    this.runeMat = new THREE.MeshBasicMaterial({ color: GOLD, side: THREE.DoubleSide });
    const ring = (r1: number, r2: number) => this.circle.add(new THREE.Mesh(new THREE.RingGeometry(r1, r2, 64), this.runeMat));
    ring(1.55, 1.65); ring(1.12, 1.17); ring(0.5, 0.54);
    for (let i = 0; i < 16; i++) {
      const a = (i / 16) * TAU;
      const rune = new THREE.Mesh(new THREE.PlaneGeometry(i % 2 ? 0.08 : 0.16, i % 2 ? 0.22 : 0.12), this.runeMat);
      rune.position.set(cos(a) * 1.38, sin(a) * 1.38, 0);
      rune.rotation.z = a;
      this.circle.add(rune);
    }
    for (let i = 0; i < 6; i++) {
      const a = (i / 6) * TAU;
      const bar = new THREE.Mesh(new THREE.PlaneGeometry(1.92, 0.04), this.runeMat);
      bar.position.set(cos(a + Math.PI / 6) * 0.56, sin(a + Math.PI / 6) * 0.56, 0);
      bar.rotation.z = a + Math.PI / 6 + Math.PI / 2;
      this.circle.add(bar);
    }
    this.circle.rotation.x = -Math.PI / 2;
    this.circle.position.y = -2.2;
    this.group.add(this.circle);
    this.group.visible = false;
  }

  set(id: MorphId | null) {
    if (id === this.id) return;
    this.id = id;
  }

  get active() { return this.m > 0.001 || this.id !== null; }

  /** Call after animate() each frame. */
  update(t: number) {
    const dt = min(0.05, max(0, t - this.last));
    this.last = t;
    // swapping spells: dissolve fully, then reform as the new one
    const want = this.id && (this.shown === null || this.shown === this.id) ? 1 : 0;
    this.m += (want ? 1 : -1) * dt / 1.1;
    this.m = min(1, max(0, this.m));
    if (this.m === 0) this.shown = this.id;
    if (want && this.shown === null) this.shown = this.id;
    const m = this.m;

    const rig = this.rig;
    const hide = smooth(m * 1.6);
    rig.head.scale.multiplyScalar(1 - hide);
    rig.head.visible = hide < 0.99;
    rig.hands.forEach((h) => h.scale.setScalar(1 - hide));
    Object.values(rig.props).forEach((g) => { if (m > 0.02) g.visible = false; });
    if (m === 0 && this.id === null) { if (this.group.visible) rig.state.expr = ""; this.group.visible = false; return; }
    this.group.visible = true;

    const def = this.shown ? MORPHS[this.shown] : null;
    this.runeMat.color.set(def?.accent ?? GOLD);
    const cs = smooth(m);
    this.circle.scale.setScalar(0.2 + cs * 0.8);
    this.circle.rotation.z = t * 0.6;
    this.circle.visible = cs > 0.02;

    for (let i = 0; i < N; i++) {
      const p = this.ps[i];
      p.s = 1; p.c = null;
      if (def) def.shape(p, t, this.palette);
      // staggered: the top of the goblin lifts off first
      const delay = (1 - p.k) * 0.45;
      const e = smooth((m - delay * (1 - m)) / (1 - delay * 0.5));
      const v = this.pos[i].copy(this.src[i]).lerp(p.out, e);
      // swirl upward mid-flight
      const swirl = sin(e * Math.PI);
      if (swirl > 0.001) {
        v.applyAxisAngle(new THREE.Vector3(0, 1, 0), swirl * (1.2 + p.r[0] * 2.5));
        v.y += swirl * (0.4 + p.r[1] * 0.8);
      }
      this.dummy.position.copy(v);
      this.dummy.scale.setScalar((1 - e) * 1 + e * p.s * (0.9 + 0.3 * p.r[2]) * (e > 0 ? 1 : 0));
      if (e <= 0.001) this.dummy.scale.setScalar(0);
      this.dummy.rotation.set(t + p.r[0] * 6, t * 0.7 + p.r[1] * 6, 0);
      this.dummy.updateMatrix();
      this.bits.setMatrixAt(i, this.dummy.matrix);
      this.col.copy(this.srcColor[i]);
      if (p.c) this.col.lerp(this.tmp.set(p.c), e);
      this.bits.setColorAt(i, this.col);
    }
    this.bits.instanceMatrix.needsUpdate = true;
    if (this.bits.instanceColor) this.bits.instanceColor.needsUpdate = true;

    // twinkles that ride along with the swarm
    const flare = 0.35 + sin(m * Math.PI) * 1.4;
    for (let j = 0; j < 48; j++) {
      const v = this.pos[(j * 13) % N], r = this.ps[(j * 13) % N].r;
      const tw = max(0, sin(t * 4 + j * 1.7));
      this.dummy.position.set(v.x + (r[2] - 0.5) * 0.5, v.y + (r[0] - 0.5) * 0.5, v.z + 0.15);
      this.dummy.rotation.set(0, 0, t * 2 + j);
      this.dummy.scale.setScalar(m * tw * flare);
      this.dummy.updateMatrix();
      this.stars.setMatrixAt(j, this.dummy.matrix);
    }
    this.stars.instanceMatrix.needsUpdate = true;
  }

  dispose() {
    this.rig.root.remove(this.group);
    this.bits.geometry.dispose();
    this.stars.geometry.dispose();
    this.circle.traverse((o) => (o as THREE.Mesh).geometry?.dispose());
  }
}
