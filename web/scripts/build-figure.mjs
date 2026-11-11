// Builds the homepage's isometric single-cylinder engine with the vendored
// Anatomy kit and writes it, with its moving-part frames and pointer anchors,
// to src/generated/engine-figure.ts. Run: node scripts/build-figure.mjs
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import * as k from "../vendor/anatomy/iso-kit.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "../src/generated/engine-figure.ts");

const WIDTH = 560;
const HEIGHT = 420;

const PLATE = { x: -122, y: -74, w: 252, d: 148, r: 8 };
const PLATE_H = 12;
const FOOT_H = 5;

const CRANK = { x: -58, y: -42, w: 86, d: 84, r: 10 };
const CRANK_TOP = 40;
const BORE = [-15, 0];
const BARREL_R = 20;
const FIN_R = 30;
const HEAD_Z = 108;
const HEAD = { x: -50, y: -35, w: 70, d: 70, r: 8 };
const HEAD_TOP = 124;
const VALVES = { in: [-28, -13], ex: [-28, 13] };
const VALVE_TOP = 146;
const MAX_LIFT = 6;
const LIFT_FRAMES = 9;
const PLUG = [4, 0];
const PLUG_TOP = 146;
const COIL = { x: 52, y: -64, w: 30, d: 26, r: 3 };
const COIL_TOP = 30;
const COIL_POST = [67, -51];
const FLY = [82, 36];
const FLY_R = 30;
const FLY_Z = 9;
const FLY_TOP = 19;
const FILTER = [-96, 30];
const SPOKE_FRAMES = 12;

const P = k.fitProjection(
  [...k.boxCorners(PLATE, -PLATE_H - FOOT_H, 0), [VALVES.in[0], VALVES.in[1], VALVE_TOP + 8], [PLUG[0], PLUG[1], PLUG_TOP + 4], [44, -34, 170]],
  WIDTH,
  HEIGHT,
  { pad: 26 },
);

const at = (point) => k.iso(point, P);
const S = (paths, style) => k.solidSvg(paths, style);
const L = (d, style) => k.lineSvg(d, style);
const D = (points, style) => k.dotsSvg(points, style);
const byDepth = (points) => [...points].sort((a, b) => k.depthOf([a[0], a[1], 0], P) - k.depthOf([b[0], b[1], 0], P));
const round = ([x, y]) => [Math.round(x * 10) / 10, Math.round(y * 10) / 10];

function plate() {
  const out = [`<path class="iso-halo" d="${k.haloOf(PLATE, -PLATE_H - FOOT_H, PLATE_H, P)}"/>`];
  for (const [x, y] of byDepth(k.corners(PLATE, 22))) {
    out.push(S(k.cylinder(x, y, 7, -PLATE_H - FOOT_H, 1.6, P, 28), { tone: "lo" }));
    out.push(S(k.cylinder(x, y, 5.4, -PLATE_H - FOOT_H + 1.6, FOOT_H - 1.6, P, 28), { tone: "mid" }));
  }
  out.push(S(k.slabOf(PLATE, -PLATE_H, PLATE_H, P, 8, 1.6), { tone: "mid" }));
  out.push(L(k.planOutline(k.insetPlan(PLATE, 5), 0, P), { tone: "lo" }));
  const screws = k.corners(PLATE, 10);
  out.push(L(screws.map(([x, y]) => k.ring(x, y, 2.2, 0, P, 16)).join(""), { tone: "lo" }));
  out.push(D(screws.map(([x, y]) => at([x, y, 0])), { size: 0.5 }));
  const strip = { x: -40, y: PLATE.y + PLATE.d - 15, w: 156, d: 7, r: 1 };
  out.push(S(k.slabOf(strip, 0, 0.6, P, 3), { tone: "lo", crease: "none" }));
  const ticks = k.topTicks(strip.x + 4, strip.x + strip.w - 4, 4, 5, strip.y + 0.8, 0.6, [2, 3.6], P);
  out.push(L(ticks.minor, { tone: "lo" }), L(ticks.major, { tone: "mid" }));
  const label = { x: -108, y: -62, w: 46, d: 24, r: 2 };
  out.push(S(k.slabOf(label, 0, 0.8, P, 4), { tone: "lo" }));
  out.push(L(k.planOutline(k.insetPlan(label, 1.6), 0.8, P), { tone: "faint" }));
  out.push(k.faceTextSvg(k.topMatrix([label.x + 6, label.y + 10], 0.8, P, "x"), "1-CYL 4T", { size: 4.6, tone: "mid" }));
  out.push(L(k.lineOnTop([label.x + 6, label.y + 16], [label.x + 30, label.y + 16], 0.8, P), { tone: "lo", free: true }));
  out.push(D([at([label.x + 3, label.y + 3, 0.8]), at([label.x + label.w - 3, label.y + label.d - 3, 0.8])], { size: 0.45 }));
  return out.join("");
}

function filter() {
  const out = [];
  const [x, y] = FILTER;
  out.push(S(k.slabOf({ x: x - 14, y: y - 14, w: 28, d: 28, r: 3 }, 0, 4, P, 4, 0.5), { tone: "mid" }));
  out.push(S(k.slabOf({ x: x + 10, y: y - 4, w: CRANK.x - x - 10, d: 8, r: 1.5 }, 16, 9, P, 3, 0.5), { tone: "mid" }));
  out.push(S(k.cylinder(x, y, 16, 4, 24, P, 48, 0.8), { tone: "mid" }));
  out.push(L([10, 16, 22].map((z) => k.sideArc(x, y, 16, z, P, 32)).join(""), { tone: "faint" }));
  out.push(S(k.cylinder(x, y, 17.5, 28, 4, P, 48, 0.6), { tone: "mid" }));
  out.push(L(k.knurl(x, y, 17.5, 28, 32, 64, P), { tone: "lo" }));
  out.push(S(k.cylinder(x, y, 3.2, 32, 3, P, 6), { tone: "mid" }));
  out.push(S(k.slabOf({ x: x - 8, y: y - 1.2, w: 16, d: 2.4, r: 1 }, 35, 1.6, P, 2), { tone: "mid", crease: "none" }));
  return out.join("");
}

function coil() {
  const out = [];
  out.push(S(k.slabOf(COIL, 0, COIL_TOP, P, 4, 1), { tone: "mid" }));
  out.push(L(k.sideSeam(COIL, 8, P, 4), { tone: "faint" }));
  out.push(S(k.slabOf(k.insetPlan(COIL, 3), COIL_TOP, 3, P, 3, 0.4), { tone: "mid" }));
  out.push(D([at([COIL.x + 6, COIL.y + 6, COIL_TOP + 3]), at([COIL.x + COIL.w - 6, COIL.y + COIL.d - 6, COIL_TOP + 3])], { size: 0.45 }));
  out.push(S(k.cylinder(COIL_POST[0], COIL_POST[1], 4, COIL_TOP + 3, 6, P, 24), { tone: "hi", crease: "lo" }));
  out.push(S(k.cylinder(COIL_POST[0], COIL_POST[1], 2, COIL_TOP + 9, 3, P, 16), { tone: "mid" }));
  return out.join("");
}

function crankcase() {
  const out = [];
  out.push(S(k.slabOf(CRANK, 0, CRANK_TOP, P, 8, 1.6), { tone: "mid" }));
  out.push(L(k.sideSeam(CRANK, 18, P, 8), { tone: "lo" }));
  out.push(L(k.planOutline(k.insetPlan(CRANK, 6), CRANK_TOP, P), { tone: "faint" }));
  const filler = [CRANK.x + 14, CRANK.y + CRANK.d - 14];
  out.push(S(k.cylinder(filler[0], filler[1], 5, CRANK_TOP, 3, P, 24), { tone: "mid" }));
  out.push(S(k.cylinder(filler[0], filler[1], 6, CRANK_TOP + 3, 2.4, P, 24, 0.3), { tone: "mid" }));
  out.push(L(k.knurl(filler[0], filler[1], 6, CRANK_TOP + 3, CRANK_TOP + 5.4, 28, P), { tone: "lo" }));
  const cover = { x: CRANK.x + CRANK.w, y: -22, w: 2, d: 44, r: 1 };
  out.push(S(k.slabOf(cover, 6, 26, P, 3, 0.5), { tone: "mid" }));
  out.push(D([[-16, 9], [16, 9], [-16, 29], [16, 29]].map(([y, z]) => at([cover.x + 2, y, z])), { size: 0.5 }));
  const bolts = k.corners(CRANK, 9);
  out.push(L(bolts.map(([x, y]) => k.ring(x, y, 2.4, CRANK_TOP, P, 16)).join(""), { tone: "lo" }));
  out.push(D(bolts.map(([x, y]) => at([x, y, CRANK_TOP])), { size: 0.5 }));
  return out.join("");
}

function barrel() {
  const out = [];
  const [x, y] = BORE;
  out.push(S(k.cylinder(x, y, 26, CRANK_TOP, 4, P, 48, 0.6), { tone: "mid" }));
  let z = CRANK_TOP + 4;
  for (let fin = CRANK_TOP + 9; fin <= HEAD_Z - 6; fin += 7) {
    out.push(S(k.cylinder(x, y, BARREL_R, z, fin - z, P, 48), { tone: "mid", crease: "none" }));
    out.push(S(k.cylinder(x, y, FIN_R, fin, 2.2, P, 56, 0.4), { tone: "hi", crease: "lo" }));
    z = fin + 2.2;
  }
  out.push(S(k.cylinder(x, y, BARREL_R, z, HEAD_Z - z, P, 48), { tone: "mid", crease: "none" }));
  return out.join("");
}

function head() {
  const out = [];
  out.push(S(k.slabOf(HEAD, HEAD_Z, HEAD_TOP - HEAD_Z, P, 8, 1.2), { tone: "mid" }));
  out.push(L(k.sideSeam(HEAD, HEAD_Z + 7, P, 8), { tone: "faint" }));
  for (const [x, y] of byDepth(k.corners(HEAD, 7))) {
    out.push(S(k.cylinder(x, y, 3, HEAD_TOP, 2.4, P, 6), { tone: "mid" }));
  }
  for (const [x, y] of byDepth(Object.values(VALVES))) {
    out.push(S(k.cylinder(x, y, 7.5, HEAD_TOP, 3, P, 32), { tone: "mid" }));
  }
  return out.join("");
}

function helix(cx, cy, radius, z0, z1, turns) {
  const count = Math.round(turns * 18);
  const back = [];
  const front = [];
  let run = [];
  let near = null;
  for (let index = 0; index <= count; index++) {
    const share = index / count;
    const angle = share * turns * Math.PI * 2;
    const point = at([cx + radius * Math.cos(angle), cy + radius * Math.sin(angle), z0 + (z1 - z0) * share]);
    const facing = k.depthOf([Math.cos(angle), Math.sin(angle), 0], P) > 0;
    if (near === null) near = facing;
    run.push(point);
    if (facing !== near) {
      (near ? front : back).push(k.pathOf(run));
      run = [point];
      near = facing;
    }
  }
  if (run.length > 1) (near ? front : back).push(k.pathOf(run));
  return { back: back.join(""), front: front.join("") };
}

function valve([x, y], lift) {
  const seat = HEAD_TOP + 3;
  const top = VALVE_TOP - lift;
  const spring = helix(x, y, 5, seat, top, 6.5);
  return [
    L(spring.back, { tone: "lo" }),
    S(k.cylinder(x, y, 1.4, seat, top + 7 - seat, P, 12), { tone: "hi", crease: "none" }),
    L(spring.front, { tone: "hi" }),
    S(k.cylinder(x, y, 6.6, top, 2.2, P, 32, 0.3), { tone: "hi", crease: "lo" }),
  ].join("");
}

function plug() {
  const [x, y] = PLUG;
  return [
    S(k.cylinder(x, y, 5.6, HEAD_TOP, 4, P, 6), { tone: "mid" }),
    S(k.cylinder(x, y, 3.4, HEAD_TOP + 4, 3, P, 24), { tone: "mid" }),
    S(k.cylinder(x, y, 3, HEAD_TOP + 7, 14, P, 24), { tone: "hi", lit: true }),
    L([HEAD_TOP + 11, HEAD_TOP + 14, HEAD_TOP + 17].map((z) => k.sideArc(x, y, 3, z, P, 16)).join(""), { tone: "faint" }),
    S(k.cylinder(x, y, 1.6, HEAD_TOP + 21, PLUG_TOP - HEAD_TOP - 21, P, 16), { tone: "hi", lit: true }),
  ].join("");
}

function flywheel() {
  const [x, y] = FLY;
  return [
    `<path class="iso-halo" d="${k.haloOf({ x: x - 24, y: y - 24, w: 48, d: 48, r: 24 }, 0, 0.1, P)}" style="filter:blur(5px)"/>`,
    S(k.cylinder(x, y, 10, 0, FLY_Z, P, 32, 0.4), { tone: "mid" }),
    S(k.cylinder(x, y, FLY_R, FLY_Z, FLY_TOP - FLY_Z, P, 72, 0.8), { tone: "hi", crease: "lo" }),
    L(k.knurl(x, y, FLY_R, FLY_Z + 1.5, FLY_TOP - 1.5, 96, P, 0.3), { tone: "lo" }),
    L(k.ring(x, y, FLY_R - 3, FLY_TOP, P, 64), { tone: "lo" }),
  ].join("");
}

function spokes(turn) {
  const [x, y] = FLY;
  const paths = [];
  for (let index = 0; index < 4; index++) {
    const angle = turn + (index * Math.PI) / 2;
    const c = Math.cos(angle);
    const s = Math.sin(angle);
    paths.push(k.lineOnTop([x + c * 9, y + s * 9], [x + c * (FLY_R - 3), y + s * (FLY_R - 3)], FLY_TOP, P));
  }
  const mark = at([x + Math.cos(turn) * (FLY_R - 6), y + Math.sin(turn) * (FLY_R - 6), FLY_TOP]);
  return { spokes: paths.join(""), mark: round(mark) };
}

function hub() {
  const [x, y] = FLY;
  return [S(k.cylinder(x, y, 8, FLY_TOP, 4, P, 32, 0.4), { tone: "hi", crease: "lo" }), S(k.cylinder(x, y, 3, FLY_TOP + 4, 2, P, 6), { tone: "mid" })].join("");
}

const wireCurve = (t) => {
  const a = [COIL_POST[0], COIL_POST[1], COIL_TOP + 12];
  const c = [44, -34, 196];
  const b = [PLUG[0], PLUG[1], PLUG_TOP];
  return a.map((value, index) => (1 - t) * (1 - t) * value + 2 * (1 - t) * t * c[index] + t * t * b[index]);
};
const wirePath = k.pathOf(Array.from({ length: 25 }, (_, index) => at(wireCurve(index / 24))));

const svg = k.figureSvg({
  width: WIDTH,
  height: HEIGHT,
  label:
    "An isometric single-cylinder four-stroke engine on a bench plate: an air filter feeding the crankcase, a finned cylinder barrel, a head with two spring-loaded valves and a spark plug, an ignition coil wired to the plug, and a flywheel.",
  body: [
    plate(),
    filter(),
    crankcase(),
    barrel(),
    head(),
    `<g data-part="valve-in">${valve(VALVES.in, 0)}</g>`,
    `<g data-part="valve-ex">${valve(VALVES.ex, 0)}</g>`,
    `<g data-part="plug">${plug()}</g>`,
    coil(),
    flywheel(),
    `<path data-part="spokes" class="iso-line" data-tone="mid" d="${spokes(0).spokes}"/>`,
    `<circle data-part="mark" class="iso-mark" r="1.6" cx="${spokes(0).mark[0]}" cy="${spokes(0).mark[1]}"/>`,
    hub(),
    `<g data-part="wire" class="iso-wire" data-tone="mid"><path class="iso-line" data-tone="mid" d="${wirePath}"/><path class="iso-line iso-spark-flow" data-tone="lit" d="${wirePath}"/></g>`,
  ],
});

const lifts = Array.from({ length: LIFT_FRAMES }, (_, index) => (index / (LIFT_FRAMES - 1)) * MAX_LIFT);
const anchors = {
  filter: round(at([FILTER[0] + 4, FILTER[1] + 12, 30])),
  valveIn: round(at([VALVES.in[0], VALVES.in[1], VALVE_TOP + 2])),
  valveEx: round(at([VALVES.ex[0], VALVES.ex[1], VALVE_TOP + 2])),
  fins: round(at([BORE[0] + FIN_R * 0.72, BORE[1] + FIN_R * 0.72, 76])),
  plug: round(at([PLUG[0], PLUG[1], PLUG_TOP])),
  coil: round(at([COIL_POST[0], COIL_POST[1], COIL_TOP + 9])),
  flywheel: round(at([FLY[0] + FLY_R * 0.6, FLY[1] + FLY_R * 0.6, FLY_TOP])),
  crankcase: round(at([CRANK.x + CRANK.w, CRANK.y + CRANK.d / 2, 18])),
};

const file = `// Generated by scripts/build-figure.mjs with the Anatomy kit. Do not edit.
export const engineFigure = ${JSON.stringify(
  {
    width: WIDTH,
    height: HEIGHT,
    svg,
    valveFrames: lifts.map((lift) => valve(VALVES.in, lift)),
    exhaustFrames: lifts.map((lift) => valve(VALVES.ex, lift)),
    spokeFrames: Array.from({ length: SPOKE_FRAMES }, (_, index) => spokes((index / SPOKE_FRAMES) * (Math.PI / 2))),
    anchors,
  },
  null,
  0,
)} as const;
export type EngineAnchor = keyof typeof engineFigure.anchors;
`;
mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, file);
writeFileSync(join(dirname(OUT), "anatomy.css"), `/* Generated from the Anatomy kit's ISO_CSS. Do not edit. */\n${k.ISO_CSS.trim()}\n`);
console.log("wrote", OUT, `${(file.length / 1024).toFixed(1)} KB`);
