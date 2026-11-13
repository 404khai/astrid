import * as k from "../../vendor/anatomy/iso-kit.mjs";
import type { Shape } from "./shapes";

/** ADR 0036 3D solids: one fixed Anatomy camera over the 1000 × 640 canvas,
 * world origin at its centre, x/y on the ground and z up. */
export type Camera = { azimuth: number; elevation: number };
export const defaultCamera: Camera = { azimuth: k.AZIMUTH, elevation: k.ELEVATION };
const projectionOf = (cam: Camera) => ({ origin: [500, 380], scale: 1, azimuth: cam.azimuth, elevation: cam.elevation });
type P3 = [number, number, number];
type P2 = [number, number];

export const project = (p: P3, cam: Camera = defaultCamera): P2 => k.iso(p, projectionOf(cam));
const depth = (p: P3, cam: Camera): number => k.depthOf(p, projectionOf(cam));

const rad = Math.PI / 180;
/** Toward the viewer, and the light the kit shades with, for a camera. */
function lighting(cam: Camera) {
  const A = cam.azimuth * rad;
  const E = cam.elevation * rad;
  const view: P3 = [Math.cos(A) * Math.cos(E), Math.sin(A) * Math.cos(E), Math.sin(E)];
  const l: P3 = [-Math.sin(A), Math.cos(A), 0.7];
  const n = Math.hypot(...l);
  return { view, light: l.map((v) => v / n) as P3 };
}
const dot = (a: P3, b: P3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const add = (a: P3, b: P3, s = 1): P3 => [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s];

function hull(points: P2[]): P2[] {
  const p = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const cross = (o: P2, a: P2, b: P2) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const half = (list: P2[]) => {
    const out: P2[] = [];
    for (const q of list) {
      while (out.length >= 2 && cross(out[out.length - 2], out[out.length - 1], q) <= 0) out.pop();
      out.push(q);
    }
    out.pop();
    return out;
  };
  return [...half(p), ...half(p.reverse())];
}

const axes: Record<string, [P3, P3, P3]> = {
  z: [[0, 0, 1], [1, 0, 0], [0, 1, 0]],
  x: [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
  y: [[0, 1, 0], [1, 0, 0], [0, 0, 1]],
};

/** A cylinder, cone or taper along any world axis, shaded like the kit's solids. */
function cylinderPaths(base: P3, r1: number, r2: number, h: number, axis: string, cam: Camera) {
  const { view: VIEW, light: LIGHT } = lighting(cam);
  const project2 = (p: P3) => project(p, cam);
  const [a, u, v] = axes[axis] ?? axes.z;
  const steps = 36;
  const normals: P3[] = Array.from({ length: steps }, (_, i) => {
    const t = (i / steps) * Math.PI * 2;
    return add([0, 0, 0], add(add([0, 0, 0], u, Math.cos(t)), v, Math.sin(t)));
  });
  const near = normals.map((n) => project2(add(base, n, r1)));
  const far = normals.map((n) => project2(add(add(base, a, h), n, r2)));
  const shades: string[][] = Array.from({ length: k.SHADES }, () => []);
  normals.forEach((n, i) => {
    const mid = add(n, normals[(i + 1) % steps]);
    if (dot(mid, VIEW) <= 0) return;
    const lit = (dot(mid, LIGHT) / Math.hypot(...mid) + 1) / 2;
    const bucket = Math.min(k.SHADES - 1, Math.max(0, Math.floor(lit * k.SHADES)));
    const j = (i + 1) % steps;
    shades[bucket].push(k.pathOf([near[i], near[j], far[j], far[i]], true));
  });
  const cap = dot(a, VIEW) > 0 ? far : near;
  return {
    fill: k.pathOf(hull([...near, ...far]), true),
    shades: shades.map((list) => list.join("")),
    top: k.pathOf(cap, true),
    outline: k.pathOf(hull([...near, ...far]), true),
    crease: k.pathOf(cap, true),
  };
}

/** World bounds: x0..x1, y0..y1, z0..z1. */
export type Bounds = [number, number, number, number, number, number];
export type Solid = { svg: string; anchor: P2; labelAt: P2; depth: number; world: P3; bounds: Bounds };

/** Builds a slab or cylinder with the kit; null for flat shapes. */
export function solid(s: Shape, cam: Camera = defaultCamera): Solid | null {
  const z = s.z ?? 0;
  if (s.shape === "slab") {
    const plan = { x: s.x!, y: s.y!, w: s.w!, d: s.d as number, r: Math.min(s.r ?? 0, s.w! / 2, (s.d as number) / 2) };
    const paths = k.slabOf(plan, z, s.h!, projectionOf(cam), 8, plan.r > 2 ? 1.2 : 0);
    const centre: P3 = [plan.x + plan.w / 2, plan.y + plan.d / 2, z + s.h!];
    const top = project(centre, cam);
    return { svg: k.solidSvg(paths, { tone: "hi" }), anchor: top, labelAt: [top[0], top[1] - 16], depth: depth([centre[0], centre[1], z + s.h! / 2], cam), world: centre, bounds: [plan.x, plan.x + plan.w, plan.y, plan.y + plan.d, z, z + s.h!] };
  }
  if (s.shape === "cylinder") {
    const axis = s.axis ?? "z";
    const [a] = axes[axis];
    const base: P3 = [s.x!, s.y!, z];
    const r2 = s.r2 ?? s.radius!;
    const paths = cylinderPaths(base, s.radius!, r2, s.h!, axis, cam);
    const end = add(base, a, s.h!);
    const mid = add(base, a, s.h! / 2);
    const world = axis === "z" ? end : add(mid, [0, 0, 1], Math.max(s.radius!, r2));
    const tip = project(world, cam);
    const r = Math.max(s.radius!, r2);
    const bounds: Bounds =
      axis === "x" ? [s.x!, s.x! + s.h!, s.y! - r, s.y! + r, z - r, z + r]
      : axis === "y" ? [s.x! - r, s.x! + r, s.y!, s.y! + s.h!, z - r, z + r]
      : [s.x! - r, s.x! + r, s.y! - r, s.y! + r, z, z + s.h!];
    return { svg: k.solidSvg(paths, { tone: "hi" }), anchor: tip, labelAt: [tip[0], tip[1] - 16], depth: depth(mid, cam), world, bounds };
  }
  const pts = s.points as unknown as number[][] | undefined;
  if ((s.shape === "line" || s.shape === "polygon") && pts && pts.every((q) => q.length === 3)) {
    const closed = s.shape === "polygon";
    const flat = pts.map((q) => project(q as P3, cam));
    const colour = paint(s.stroke) ?? "var(--anatomy-hi)";
    const fill = closed ? paint(s.fill) ?? "none" : "none";
    const svg = `<path class="cv-wire" d="${k.pathOf(flat, closed)}" fill="${fill}" stroke="${colour}" stroke-width="${s.width ?? 1.4}"${s.dashed ? ' stroke-dasharray="5 5"' : ""}/>`;
    // A trace lies on the surface below it: sort it just above that surface.
    const mid = pts[Math.floor(pts.length / 2)] as P3;
    const end = flat[flat.length - 1];
    const along = (i: number) => pts.map((q) => q[i]);
    const bounds: Bounds = [Math.min(...along(0)), Math.max(...along(0)), Math.min(...along(1)), Math.max(...along(1)), Math.min(...along(2)), Math.max(...along(2))];
    return { svg, anchor: end, labelAt: [end[0], end[1] - 12], depth: depth(mid, cam), world: mid, bounds };
  }
  return null;
}

/** Painter's order for solids: anything resting on top of another part (its
 * footprint overlaps and it starts where the other ends) is drawn after it;
 * otherwise farther parts first. A depth-ordered topological sort. */
export function paintOrder<T extends { built: Solid; n: number }>(items: T[]): T[] {
  const on = (a: Bounds, b: Bounds) =>
    a[0] < b[1] - 0.5 && b[0] < a[1] - 0.5 && a[2] < b[3] - 0.5 && b[2] < a[3] - 0.5 && a[4] >= b[5] - 0.5;
  const after = items.map((a) => items.filter((b) => b !== a && on(a.built.bounds, b.built.bounds) && !on(b.built.bounds, a.built.bounds)));
  const left = new Set(items.map((_, i) => i));
  const done = new Set<T>();
  const out: T[] = [];
  while (left.size) {
    let pick = -1;
    for (const i of left) {
      if (!after[i].every((b) => done.has(b))) continue;
      if (pick < 0 || items[i].built.depth < items[pick].built.depth || (items[i].built.depth === items[pick].built.depth && items[i].n < items[pick].n)) pick = i;
    }
    if (pick < 0) pick = [...left][0];
    left.delete(pick);
    done.add(items[pick]);
    out.push(items[pick]);
  }
  return out;
}

const named: Record<string, string> = {
  ink: "#e6e6e6", paper: "#1d1d1f", blue: "#6575ff", violet: "#9b7bff", pink: "#ec6a8a", red: "#f94447",
  orange: "#ff8a3d", yellow: "#f7c600", green: "#30a46c", teal: "#00c2b8", grey: "#8a8a8a", none: "none",
};
/** Colours were validated by the runtime: a palette name or a #hex. */
const paint = (c?: string) => (c ? named[c] ?? (/^#[0-9a-f]{3,8}$/i.test(c) ? c : undefined) : undefined);

/** Text, points and arrows given in world coordinates land on the canvas. */
export function flatten(s: Shape, cam: Camera = defaultCamera): Shape {
  const p = (q: number[]) => (q.length === 3 ? project(q as P3, cam) : (q as P2));
  if ((s.shape === "text" || s.shape === "point") && s.z !== undefined) {
    const [x, y] = project([s.x!, s.y!, s.z], cam);
    return { ...s, x, y, z: undefined };
  }
  if (s.shape === "arrow") return { ...s, from: p(s.from as number[]), to: p(s.to as number[]) };
  return s;
}

export const ISO_CSS: string = k.ISO_CSS;
