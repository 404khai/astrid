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

export function geometry(form: Form, corners: Corners = "curved"): Geometry {
  const sharp = corners === "sharp";
  if (form === "classic" && !sharp) return { top: TOP_BAR, bottom: BOTTOM_BAR, full: FULL, window: WINDOW };
  const key = `${form}:${corners}`;
  const hit = cache.get(key);
  if (hit) return hit;
  const outer = mirrorX(HALF[form]);
  const inner = under(outer, T, sharp);
  const bend = sharp ? 0 : 12, side = sharp ? 0 : 28, wall = sharp ? 0 : 20;
  const band = (o: P[], i: P[]) => rounded([...o, ...i.slice().reverse()], (k) => (k === 0 || k === o.length - 1 || k === o.length || k === o.length * 2 - 1 ? 1 : bend));
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
    full: rounded(fullPts, (k) => (sides.has(k) ? side : bend)),
    window: rounded(winPts, (k) => (walls.has(k) ? wall : bend)),
  };
  cache.set(key, g);
  return g;
}
