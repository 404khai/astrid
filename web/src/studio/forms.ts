// Forms: the silhouette family of the mark. Classic is the original artwork; the others are built here from
// a half profile of the top arch's outer edge, mirrored left/right and top/bottom, so every form gets
// arches, a joined full shape and a dual eye window with the same rules.
import { BOTTOM_BAR, FULL, TOP_BAR, WINDOW } from "./faces";

type P = [number, number];
export const FORMS = ["classic", "round", "block", "peak", "steps", "visor", "valley", "dome", "capsule", "keystone", "crest", "rampart", "ticket", "lynx", "bracket", "sprocket"] as const;
export type Form = (typeof FORMS)[number];
export const FORM_LABEL: Record<Form, string> = { classic: "Classic", round: "Round", block: "Block", peak: "Peak", steps: "Steps", visor: "Visor", valley: "Valley",
  dome: "Dome", capsule: "Capsule", keystone: "Keystone", crest: "Crest", rampart: "Rampart", ticket: "Ticket", lynx: "Lynx", bracket: "Bracket", sprocket: "Sprocket" };

const L = 51.3, R = 460.2, MX = (L + R) / 2, MY = 255.7, T = 50;
const rise = (x0: number, x1: number, y0: number, y1: number, n = 10): P[] =>
  Array.from({ length: n + 1 }, (_, i) => { const k = i / n, s = k * k * (3 - 2 * k); return [x0 + (x1 - x0) * k, y0 + (y1 - y0) * s] as P; });

/** Left half of the top arch's outer edge, from the left end to the centre line. */
const HALF: Record<Form, P[]> = {
  // classic as straight lines, for sharp corners; curved classic uses the original artwork
  classic: [[L, 151.5], [110.4, 151.5], [169.6, 92.3], [MX, 92.3]],
  round: [[L, 151.5], ...rise(105, 205, 151.5, 92.3, 12), [MX, 92.3]],
  // risers sit outside the eyes so the inner edge never cuts into them
  block: [[L, 151.5], [108, 151.5], [108, 92.3], [MX, 92.3]],
  peak: [[L, 151.5], [125, 151.5], [MX, 92.3]],
  steps: [[L, 151.5], [100, 151.5], [100, 122], [158, 122], [158, 92.3], [MX, 92.3]],
  visor: [[L, 118], [MX, 118]],
  valley: [[L, 92.3], [140, 92.3], [196, 138], [MX, 138]],
  dome: Array.from({ length: 17 }, (_, i) => { const x = L + ((MX - L) * i) / 16; return [x, 151.5 - 59.2 * Math.sin((Math.PI / 2) * (i / 16)) ** 0.6] as P; }),
  capsule: [[L, 151.5], ...rise(L + 4, 128, 151.5, 92.3, 10).slice(1), [MX, 92.3]],
  keystone: [[L, 151.5], [88, 151.5], [168, 100], [MX, 100]],
  crest: [[L, 151.5], [96, 151.5], [124, 96], [158, 130], [MX, 88]],
  rampart: [[L, 151.5], [92, 151.5], [92, 96], [132, 96], [132, 124], [172, 124], [172, 96], [MX, 96]],
  ticket: [[L, 151.5], [98, 151.5], [146, 98], [204, 98], [216, 108], [232, 118], [MX, 120]],
  lynx: [[L, 151.5], [66, 151.5], [96, 74], [132, 120], [MX, 120]],
  bracket: [[L, 92.3], [94, 92.3], [94, 138], [MX, 138]],
  sprocket: [[L, 151.5], [92, 151.5], [92, 104], [124, 104], [124, 126], [156, 126], [156, 104], [188, 104], [188, 126], [220, 126], [220, 104], [MX, 104]],
};

const mirrorX = (pts: P[]): P[] => {
  const right = pts.slice().reverse().map(([x, y]) => [2 * MX - x, y] as P);
  return [...pts, ...(Math.abs(pts[pts.length - 1][0] - MX) < 0.01 ? right.slice(1) : right)];
};
const flipY = (pts: P[]) => pts.map(([x, y]) => [x, 2 * MY - y] as P);

/**
 * The inner edge of a band of even thickness d under the outer edge: at each x, the lowest point a disc of
 * radius d reaches when rolled along the outline. Unlike a mitred offset it never crosses itself, even past
 * narrow teeth, and its inner corners come out rounded.
 */
function under(outer: P[], d: number, sharp = false, step = 2): P[] {
  // sharp rolls an octagon instead of a disc: the same thickness on flats, 45° slopes and risers, with crisp corners
  const flat = d * (Math.SQRT2 - 1);
  const reach = (dx: number) => (sharp ? (Math.abs(dx) <= flat ? d : d - (Math.abs(dx) - flat)) : Math.sqrt(d * d - dx * dx));
  const dense: P[] = [];
  outer.forEach((p, i) => {
    const q = outer[i + 1];
    if (!q) return void dense.push(p);
    const n = Math.max(1, Math.ceil(Math.hypot(q[0] - p[0], q[1] - p[1]) / step));
    for (let k = 0; k < n; k++) dense.push([p[0] + ((q[0] - p[0]) * k) / n, p[1] + ((q[1] - p[1]) * k) / n]);
  });
  // the outline runs left to right, so a sliding window over x finds the points within reach
  const out: P[] = [];
  let lo = 0;
  for (let x = L; x <= R + 0.01; x += step) {
    while (lo < dense.length && dense[lo][0] < x - d) lo++;
    let y = -Infinity;
    for (let i = lo; i < dense.length && dense[i][0] <= x + d; i++) y = Math.max(y, dense[i][1] + reach(dense[i][0] - x));
    out.push([Math.min(x, R), y]);
  }
  // drop points that sit on a straight run, so the path stays small
  return out.filter((p, i) => {
    const a = out[i - 1], b = out[i + 1];
    return !a || !b || Math.abs((p[1] - a[1]) * (b[0] - p[0]) - (b[1] - p[1]) * (p[0] - a[0])) > 0.5;
  });
}

/** A closed polygon with each corner rounded (radius per corner, clamped to half of the shorter neighbouring edge). */
function rounded(pts: P[], radius: (i: number) => number): string {
  const f = (n: number) => Math.round(n * 10) / 10;
  const n = pts.length;
  let d = "";
  pts.forEach((c, i) => {
    const a = pts[(i - 1 + n) % n], b = pts[(i + 1) % n];
    const la = Math.hypot(a[0] - c[0], a[1] - c[1]), lb = Math.hypot(b[0] - c[0], b[1] - c[1]);
    const r = Math.min(radius(i), la / 2, lb / 2);
    const p1: P = [c[0] + ((a[0] - c[0]) / la) * r, c[1] + ((a[1] - c[1]) / la) * r];
    const p2: P = [c[0] + ((b[0] - c[0]) / lb) * r, c[1] + ((b[1] - c[1]) / lb) * r];
    d += `${i ? "L" : "M"}${f(p1[0])} ${f(p1[1])}Q${f(c[0])} ${f(c[1])} ${f(p2[0])} ${f(p2[1])}`;
  });
  return d + "Z";
}

export type Corners = "curved" | "sharp";
export interface Geometry { top: string; bottom: string; full: string; window: string }
const cache = new Map<string, Geometry>();

/**
 * Fluffy: walk the outline, cut it into even steps, and join the steps with small arcs that bulge outward,
 * so every edge turns into a row of soft cloud bumps.
 */
function fluff(pts: P[], step = 26): string {
  const f = (n: number) => Math.round(n * 10) / 10;
  const n = pts.length, seg = pts.map((p, i) => Math.hypot(pts[(i + 1) % n][0] - p[0], pts[(i + 1) % n][1] - p[1]));
  const per = seg.reduce((a, b) => a + b, 0), count = Math.max(6, Math.round(per / step)), len = per / count;
  const at = (d: number): P => {
    let i = 0;
    while (d > seg[i] && i < n - 1) d -= seg[i++];
    const a = pts[i], b = pts[(i + 1) % n], k = seg[i] ? d / seg[i] : 0;
    return [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k];
  };
  // shoelace sign tells which way the outline turns, so the arcs bulge out rather than in
  const area = pts.reduce((a, p, i) => a + p[0] * pts[(i + 1) % n][1] - pts[(i + 1) % n][0] * p[1], 0);
  const sweep = area > 0 ? 1 : 0, r = f(len * 0.58);
  const marks = Array.from({ length: count }, (_, i) => at(i * len));
  return `M${f(marks[0][0])} ${f(marks[0][1])}` + marks.map((_, i) => { const q = marks[(i + 1) % count]; return `A${r} ${r} 0 0 ${sweep} ${f(q[0])} ${f(q[1])}`; }).join("") + "Z";
}

export function geometry(form: Form, corners: Corners = "curved", fluffy = false): Geometry {
  const sharp = corners === "sharp";
  if (form === "classic" && !sharp && !fluffy) return { top: TOP_BAR, bottom: BOTTOM_BAR, full: FULL, window: WINDOW };
  const key = `${form}:${corners}:${fluffy}`;
  const hit = cache.get(key);
  if (hit) return hit;
  const outer = mirrorX(HALF[form]);
  const inner = under(outer, T, sharp);
  const bend = sharp ? 0 : 12, side = sharp ? 0 : 28, wall = sharp ? 0 : 20;
  const band = (o: P[], i: P[]) => fluffy ? fluff([...o, ...i.slice().reverse()]) : rounded([...o, ...i.slice().reverse()], (k) => (k === 0 || k === o.length - 1 || k === o.length || k === o.length * 2 - 1 ? 1 : bend));
  const fullPts = [...outer, ...flipY(outer).reverse()];
  const sides = new Set([0, outer.length - 1, outer.length, fullPts.length - 1]);
  // the dual window: the inner edges, walled in at the arch-end thickness
  const wl = L + T, wr = R - T;
  const trim = (pts: P[]) => {
    const yAt = (x: number) => {
      const i = pts.findIndex((p) => p[0] >= x), a = pts[Math.max(0, i - 1)], b = pts[i];
      return b[0] === a[0] ? b[1] : a[1] + ((b[1] - a[1]) * (x - a[0])) / (b[0] - a[0]);
    };
    const inside = pts.filter(([x]) => x > wl + 1 && x < wr - 1);
    return [[wl, yAt(wl)] as P, ...inside, [wr, yAt(wr)] as P];
  };
  const wTop = trim(inner), wBot = flipY(wTop).reverse();
  const winPts = [...wTop, ...wBot];
  const walls = new Set([0, wTop.length - 1, wTop.length, winPts.length - 1]);
  const g: Geometry = {
    top: band(outer, inner),
    bottom: band(flipY(outer), flipY(inner)),
    full: fluffy ? fluff(fullPts) : rounded(fullPts, (k) => (sides.has(k) ? side : bend)),
    window: fluffy ? fluff(winPts, 22) : rounded(winPts, (k) => (walls.has(k) ? wall : bend)),
  };
  cache.set(key, g);
  return g;
}

/** Shoelace area: the sign tells which way an outline winds. */
const area = (pts: P[]) => pts.reduce((a, p, i) => a + p[0] * pts[(i + 1) % pts.length][1] - pts[(i + 1) % pts.length][0] * p[1], 0);

/** Astrid's 512 box into open-bot-avatar's 0 0 100 100 box: 8..92 wide, centred. */
export const AVATAR_K = 84 / (R - L);
export const toAvatar = ([x, y]: P): P => [(x - L) * AVATAR_K + 8, (y - MY) * AVATAR_K + 50];

/**
 * Astrid as an open-bot-avatar body for the fluffy 3D renderer, one path in its 0 0 100 100 box:
 * the full silhouette, the two arches as separate pieces, or dual, the full shape with the eye
 * window as a hole (wound the other way, as its nonzero fill needs). The eyes sit where Astrid's do.
 */
export function avatarDefinition(form: Form, corners: Corners, style: "arches" | "full" | "dual" = "full") {
  const k = AVATAR_K, sharp = corners === "sharp";
  const bend = sharp ? 0 : 12 * k, side = sharp ? 0 : 28 * k, wall = sharp ? 0 : 20 * k;
  const outer = mirrorX(HALF[form]);
  const full = [...outer, ...flipY(outer).reverse()];
  const sides = new Set([0, outer.length - 1, outer.length, full.length - 1]);
  const fullPath = rounded(full.map(toAvatar), (i) => (sides.has(i) ? side : bend));
  let path = fullPath;
  const inner = under(outer, T, sharp);
  if (style !== "full") {
    if (style === "arches") {
      const band = (o: P[], i: P[]) => {
        const pts = [...o, ...i.slice().reverse()];
        return rounded(pts.map(toAvatar), (j) => (j === 0 || j === o.length - 1 || j === o.length || j === o.length * 2 - 1 ? 0.2 : bend));
      };
      path = band(outer, inner) + band(flipY(outer), flipY(inner));
    } else {
      const wl = L + T, wr = R - T;
      const yAt = (x: number) => {
        const i = inner.findIndex((p) => p[0] >= x), a = inner[Math.max(0, i - 1)], b = inner[i];
        return b[0] === a[0] ? b[1] : a[1] + ((b[1] - a[1]) * (x - a[0])) / (b[0] - a[0]);
      };
      const wTop: P[] = [[wl, yAt(wl)], ...inner.filter(([x]) => x > wl + 1 && x < wr - 1), [wr, yAt(wr)]];
      let win = [...wTop, ...flipY(wTop).reverse()];
      const walls = new Set([0, wTop.length - 1, wTop.length, win.length - 1]);
      let radius = (j: number) => (walls.has(j) ? wall : bend);
      if (Math.sign(area(win)) === Math.sign(area(full))) {
        const n = win.length, r0 = radius;
        win = win.slice().reverse();
        radius = (j: number) => r0(n - 1 - j);
      }
      path = fullPath + rounded(win.map(toAvatar), radius);
    }
  }
  return {
    version: 1 as const, id: `astrid-${form}-${style}`, name: "Astrid",
    body: { path },
    // the package checks that its eyes sit on fur, and arches and dual have none behind Astrid's eyes:
    // there it gets a small valid pair inside the top arch, and FurBot moves the real eyes into the gap
    eyes: style === "full"
      ? { x: 50, y: 50, spacing: Math.round(102.9 * k * 10) / 10, scale: 1.8 }
      : STAND_IN,
  };
}

const STAND_IN = { x: 50, y: 21, spacing: 6, scale: 0.6 };
const standIns = new Map<string, typeof STAND_IN>();

/**
 * For arches and dual: the first spot along the top arch where the package accepts the small
 * stand-in eyes, found with its own validator and kept per shape.
 */
export function withStandIn<D extends { id: string; body: { path: string }; eyes: typeof STAND_IN }>(def: D, valid: (d: D) => boolean): D {
  if (def.eyes !== STAND_IN) return def;
  const hit = standIns.get(def.body.path);
  if (hit) return { ...def, eyes: hit };
  for (const x of [50, 38, 62, 26, 74, 16, 84, 12, 88]) {
    for (let y = 8; y <= 92; y += 0.5) {
      const eyes = { ...STAND_IN, x, y };
      if (valid({ ...def, eyes })) { standIns.set(def.body.path, eyes); return { ...def, eyes }; }
    }
  }
  return def;
}
