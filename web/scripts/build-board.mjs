// Builds the whiteboard figure: an Anatomy-kit stand plus two particle models
// in the same isometric camera, a DNA double helix and a human heart, with the
// heart's parts labeled. Writes src/generated/board-figure.ts.
// Run: node scripts/build-board.mjs
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import * as k from "../vendor/anatomy/iso-kit.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "../src/generated/board-figure.ts");
const WIDTH = 560;
const HEIGHT = 400;
const N = 1100;

let seed = 7;
const random = () => {
  seed = (seed * 16807) % 2147483647;
  return (seed - 1) / 2147483646;
};

// ── the stand: plate on feet and a turntable, drawn with the kit ──
const PLATE = { x: -96, y: -76, w: 192, d: 152, r: 8 };
const TABLE_R = 40;
const TABLE_TOP = 10;
const P = k.fitProjection(
  [...k.boxCorners(PLATE, -17, 0), [0, 0, 172], [-60, -60, 120], [60, 60, 120]],
  WIDTH,
  HEIGHT,
  { pad: 22 },
);
const S = (paths, style) => k.solidSvg(paths, style);
const L = (d, style) => k.lineSvg(d, style);
const at = (point) => k.iso(point, P);
const byDepth = (points) => [...points].sort((a, b) => k.depthOf([a[0], a[1], 0], P) - k.depthOf([b[0], b[1], 0], P));

function stand() {
  const out = [`<path class="iso-halo" d="${k.haloOf(PLATE, -17, 12, P)}"/>`];
  for (const [x, y] of byDepth(k.corners(PLATE, 20))) {
    out.push(S(k.cylinder(x, y, 7, -17, 1.6, P, 28), { tone: "lo" }));
    out.push(S(k.cylinder(x, y, 5.4, -15.4, 3.4, P, 28), { tone: "mid" }));
  }
  out.push(S(k.slabOf(PLATE, -12, 12, P, 8, 1.6), { tone: "mid" }));
  out.push(L(k.planOutline(k.insetPlan(PLATE, 5), 0, P), { tone: "lo" }));
  const screws = k.corners(PLATE, 10);
  out.push(L(screws.map(([x, y]) => k.ring(x, y, 2.2, 0, P, 16)).join(""), { tone: "lo" }));
  out.push(k.dotsSvg(screws.map(([x, y]) => at([x, y, 0])), { size: 0.5 }));
  const strip = { x: -70, y: PLATE.y + PLATE.d - 14, w: 140, d: 6, r: 1 };
  out.push(S(k.slabOf(strip, 0, 0.6, P, 3), { tone: "lo", crease: "none" }));
  const ticks = k.topTicks(strip.x + 4, strip.x + strip.w - 4, 4, 5, strip.y + 0.8, 0.6, [2, 3.6], P);
  out.push(L(ticks.minor, { tone: "lo" }), L(ticks.major, { tone: "mid" }));
  out.push(S(k.cylinder(0, 0, TABLE_R + 3, 0, 3, P, 72, 0.5), { tone: "mid" }));
  out.push(S(k.cylinder(0, 0, TABLE_R, 3, TABLE_TOP - 3, P, 72, 0.6), { tone: "hi", crease: "lo" }));
  out.push(L(k.knurl(0, 0, TABLE_R, 4, TABLE_TOP - 1, 120, P, 0.3), { tone: "lo" }));
  out.push(L(k.ring(0, 0, TABLE_R - 5, TABLE_TOP, P, 72), { tone: "lo" }));
  out.push(L(k.ring(0, 0, 4, TABLE_TOP, P, 24), { tone: "mid" }));
  return out.join("");
}

// ── DNA: two strands and their rungs, as particles ──
const DNA_Z = [30, 160];
const DNA_R = 24;
const TURNS = 2.2;
const RUNGS = 24;
function dna() {
  const points = [];
  const strandCount = (N - RUNGS * 20) / 2;
  for (const [offset, color] of [[0, 0], [Math.PI, 1]]) {
    for (let index = 0; index < strandCount; index++) {
      const s = index / (strandCount - 1);
      const theta = s * TURNS * Math.PI * 2 + offset;
      const z = DNA_Z[0] + s * (DNA_Z[1] - DNA_Z[0]);
      points.push([DNA_R * Math.cos(theta), DNA_R * Math.sin(theta), z, color, -1]);
    }
  }
  const perRung = 20;
  for (let rung = 0; rung < RUNGS; rung++) {
    const s = (rung + 0.5) / RUNGS;
    const theta = s * TURNS * Math.PI * 2;
    const z = DNA_Z[0] + s * (DNA_Z[1] - DNA_Z[0]);
    for (let index = 0; index < perRung; index++) {
      const u = (index + 0.5) / perRung * 2 - 1;
      const half = u < 0 ? 0 : 1;
      points.push([DNA_R * u * Math.cos(theta), DNA_R * u * Math.sin(theta), z, 2 + ((rung + half * 2) % 4), rung]);
    }
  }
  return points;
}
const rungZ = Array.from({ length: RUNGS }, (_, rung) => DNA_Z[0] + ((rung + 0.5) / RUNGS) * (DNA_Z[1] - DNA_Z[0]));

// ── the heart: an implicit surface, plus its great vessels ──
const HEART_SCALE = 50;
const HEART_LIFT = 92;
const PARTS = [
  { id: "rv", name: "Right ventricle", detail: "Pumps used, oxygen-poor blood out to the lungs.", color: 6 },
  { id: "lv", name: "Left ventricle", detail: "The thickest wall: it pushes blood round the whole body.", color: 7 },
  { id: "ra", name: "Right atrium", detail: "Collects blood coming back from the body.", color: 8 },
  { id: "la", name: "Left atrium", detail: "Receives fresh, oxygen-rich blood from the lungs.", color: 9 },
  { id: "ao", name: "Aorta", detail: "The body's main artery, arching up and over.", color: 10 },
  { id: "pa", name: "Pulmonary artery", detail: "Carries blood from the right ventricle to the lungs.", color: 11 },
  { id: "vc", name: "Superior vena cava", detail: "Returns blood from the head and arms.", color: 12 },
];
const heartF = (x, y, z) => {
  const a = x * x + 2.25 * y * y + z * z - 1;
  return a * a * a - x * x * z * z * z - 0.1125 * y * y * z * z * z;
};
function surfacePoint() {
  for (;;) {
    const u = random() * 2 - 1;
    const phi = random() * Math.PI * 2;
    const r = Math.sqrt(1 - u * u);
    const dir = [r * Math.cos(phi), r * Math.sin(phi) * 0.7, u];
    let lo = 0;
    let hi = 2;
    if (heartF(dir[0] * hi, dir[1] * hi, dir[2] * hi) <= 0) continue;
    for (let step = 0; step < 30; step++) {
      const mid = (lo + hi) / 2;
      if (heartF(dir[0] * mid, dir[1] * mid, dir[2] * mid) > 0) hi = mid;
      else lo = mid;
    }
    return dir.map((v) => v * lo);
  }
}
function tube(curve, radius, count, part) {
  const points = [];
  for (let index = 0; index < count; index++) {
    const s = random();
    const angle = random() * Math.PI * 2;
    const [x, y, z] = curve(s);
    const [x2, y2, z2] = curve(Math.min(1, s + 0.01));
    const t = [x2 - x, y2 - y, z2 - z];
    const length = Math.hypot(...t) || 1;
    const n1 = Math.abs(t[2] / length) > 0.9 ? [1, 0, 0] : [0, 0, 1];
    const b = [t[1] * n1[2] - t[2] * n1[1], t[2] * n1[0] - t[0] * n1[2], t[0] * n1[1] - t[1] * n1[0]];
    const bl = Math.hypot(...b);
    const c = [(t[1] * b[2] - t[2] * b[1]) / length, (t[2] * b[0] - t[0] * b[2]) / length, (t[0] * b[1] - t[1] * b[0]) / length];
    const cl = Math.hypot(...c);
    points.push([
      x + radius * (Math.cos(angle) * b[0] / bl + Math.sin(angle) * c[0] / cl),
      y + radius * (Math.cos(angle) * b[1] / bl + Math.sin(angle) * c[1] / cl),
      z + radius * (Math.cos(angle) * b[2] / bl + Math.sin(angle) * c[2] / cl),
      part,
    ]);
  }
  return points;
}
function heart() {
  const out = [];
  for (let index = 0; index < 800; index++) {
    const [x, y, z] = surfacePoint();
    const part = z > 0.3 ? (x < 0 ? 2 : 3) : x < 0.12 ? 0 : 1;
    out.push([x, y, z, part]);
  }
  out.push(...tube((s) => {
    const a = s * Math.PI;
    return [0.2 - 0.45 * (1 - Math.cos(a)), 0.1, 0.6 + 0.9 * Math.sin(a) + (s > 0.5 ? -0.3 * (s - 0.5) : 0)];
  }, 0.17, 120, 4));
  out.push(...tube((s) => [-0.05 + 0.35 * s, 0.35 + 0.25 * s, 0.5 + 0.75 * s], 0.14, 100, 5));
  out.push(...tube((s) => [-0.78, -0.12, 0.55 + 0.9 * s], 0.12, 80, 6));
  return out.map(([x, y, z, part]) => [
    x * HEART_SCALE,
    y * HEART_SCALE,
    z * HEART_SCALE + HEART_LIFT,
    PARTS[part].color,
    part,
  ]);
}

const dnaPoints = dna().sort((a, b) => a[2] - b[2]);
const heartPoints = heart().sort((a, b) => a[2] - b[2]);
const centroids = PARTS.map((_, part) => {
  const mine = heartPoints.filter((point) => point[4] === part);
  return [0, 1, 2].map((axis) => mine.reduce((sum, point) => sum + point[axis], 0) / mine.length);
});
const flat = (points) => points.flatMap(([x, y, z, color, tag]) => [Math.round(x * 10) / 10, Math.round(y * 10) / 10, Math.round(z * 10) / 10, color, tag]);

const svg = k.figureSvg({
  width: WIDTH,
  height: HEIGHT,
  label: "A bench stand with a turntable, on which Astrid builds a DNA double helix and a human heart out of particles.",
  body: [stand()],
});

const file = `// Generated by scripts/build-board.mjs with the Anatomy kit. Do not edit.
export const boardFigure = ${JSON.stringify({
  width: WIDTH,
  height: HEIGHT,
  projection: P,
  svg,
  count: N,
  dna: flat(dnaPoints),
  heart: flat(heartPoints),
  rungZ: rungZ.map((z) => Math.round(z * 10) / 10),
  heartCentre: [0, 0, HEART_LIFT],
  parts: PARTS.map((part, index) => ({ ...part, centre: centroids[index].map((v) => Math.round(v * 10) / 10) })),
})} as const;
`;
mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, file);
console.log("wrote", OUT, `${(file.length / 1024).toFixed(1)} KB`, "dna", dnaPoints.length, "heart", heartPoints.length);
