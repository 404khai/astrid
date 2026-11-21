// Astrid faces: the visor mark from references/astrid.svg (two chamfered bars, two block eyes),
// re-cut per expression. All geometry lives in the mark's 512 box. Flat fills only.

export const TOP_BAR = "M459.264 151.488H413.465C405.576 151.488 398.004 148.38 392.389 142.837L349.947 100.939C344.332 95.3958 336.76 92.288 328.871 92.288H182.681C174.792 92.288 167.22 95.3959 161.605 100.939L119.163 142.837C113.548 148.38 105.976 151.488 98.0866 151.488H52.2879C51.7119 151.488 51.2639 151.872 51.2639 152.448V205.824C51.2639 206.336 51.7119 206.656 52.2879 206.656H115.567C123.489 206.656 131.09 203.522 136.711 197.938L178.041 156.878C183.662 151.294 191.262 148.16 199.185 148.16H313.272C321.189 148.16 328.785 151.289 334.405 156.866L375.803 197.95C381.423 203.527 389.018 206.656 396.935 206.656H459.264C459.84 206.656 460.224 206.336 460.224 205.824V152.448C460.224 151.872 459.84 151.488 459.264 151.488Z";
export const BOTTOM_BAR = "M430.912 304.032C430.912 303.908 430.812 303.808 430.688 303.808H397.429C389.523 303.808 381.937 306.929 376.319 312.492L334.465 353.94C328.847 359.503 321.261 362.624 313.355 362.624H199.329C191.435 362.624 183.859 359.512 178.243 353.964L136.7 312.916C131.085 307.368 123.509 304.256 115.615 304.256H52.3519C51.7119 304.256 51.3279 304.64 51.3279 305.152V358.528C51.3279 359.04 51.7119 359.36 52.3519 359.36H97.9564C105.885 359.36 113.491 362.499 119.113 368.09L161.591 410.342C167.212 415.933 174.819 419.072 182.747 419.072H328.842C336.748 419.072 344.335 415.951 349.953 410.387L392.703 368.045C398.32 362.481 405.907 359.36 413.814 359.36H459.136C459.712 359.36 460.16 359.04 460.16 358.528V305.152C460.16 304.64 459.712 304.256 459.136 304.256H431.136C431.012 304.256 430.912 304.156 430.912 304.032Z";

/** Full shape: both arches joined at the sides into one plate. The side corners are rounded at about half the arch thickness. */
export const FULL = "M79.3 151.5H98.1C106 151.5 113.5 148.4 119.2 142.8L161.6 100.9C167.2 95.4 174.8 92.3 182.7 92.3H328.9C336.8 92.3 344.3 95.4 350 100.9L392.4 142.8C398 148.4 405.6 151.5 413.5 151.5H432.2C447.7 151.5 460.2 164 460.2 179.5V331.4C460.2 346.9 447.7 359.4 432.2 359.4H413.8C405.9 359.4 398.3 362.5 392.7 368L350 410.4C344.3 416 336.7 419.1 328.8 419.1H182.7C174.8 419.1 167.2 415.9 161.6 410.3L119.1 368.1C113.5 362.5 105.9 359.4 98 359.4H79.3C63.8 359.4 51.3 346.9 51.3 331.4V179.5C51.3 164 63.8 151.5 79.3 151.5Z";

/** Dual: the eye window inside the full shape. Its side walls are as thick as the arch ends, and its corners use the same radius as the arch bends. */
export const WINDOW = "M120 206.7C126.5 206.7 132 204 136.7 197.9L178 156.9C183.7 151.3 191.3 148.2 199.2 148.2H312.3C320.2 148.2 327.8 151.3 333.5 156.9L374.8 197.9C379.5 204 385 206.7 391.5 206.7C399.5 206.7 405.2 212 405.2 222V289C405.2 299 399.5 304.3 391.5 304.3C385 304.3 379.5 307 374.8 312.6L333.5 354C327.8 359.5 320.2 362.6 312.3 362.6H199.2C191.3 362.6 183.7 359.5 178 354L136.7 312.6C132 307 126.5 304.3 120 304.3C112 304.3 106.3 299 106.3 289V222C106.3 212 112 206.7 120 206.7Z";

const W = 61, H = 101, CY = 255;
const CX = { L: 204.6, R: 307.5 } as const;
const MIDX = 255.75;
const SCOPE = 140;
const BLIPS: [number, number][] = [[50, 0.75], [140, 0.5], [230, 0.82], [310, 0.42]];

export type EyeKind = "block" | "happy" | "sad-arc" | "line" | "heart" | "x" | "ring" | "dot" | "star" | "chev" | "plus" | "minus" | "divide" | "equals" | "scan" | "percent" | "sqrt" | "approx" | "lt" | "gt" | "plusminus" | "pi" | "power" | "wave" | "sync" | "radar" | "slices" | "scope" | "scope-grid" | "scope-wedge" | "bang";
/** One eye. lid/base cut the block from the top/bottom (0..1 of height) at its outer and inner corners. */
/** beam: 0..1, where the scan line sits inside a "scan" eye. */
/** ghost: extra eyes drawn in a darker shade of the eye colour (unlit stages, empty tracks). slices: glitch offsets per band, in eye widths. */
export interface Eye { kind?: EyeKind; beam?: number; ghost?: Eye[]; slices?: number[]; dx?: number; dy?: number; sx?: number; sy?: number; lid?: [number, number]; base?: [number, number] }
export interface Face { L: Eye; R: Eye }

const r = (n: number) => Math.round(n * 10) / 10;
const poly = (pts: [number, number][]) => "M" + pts.map(([x, y]) => `${r(x)} ${r(y)}`).join("L") + "Z";
const rect = (x0: number, y0: number, x1: number, y1: number) => poly([[x0, y0], [x1, y0], [x1, y1], [x0, y1]]);
/** A polyline drawn as a band k thick, as one outline. */
function stroke(pts: [number, number][], k: number) {
  const side = (sgn: number) => pts.map((p, i) => {
    const a = pts[Math.max(0, i - 1)], b = pts[Math.min(pts.length - 1, i + 1)];
    const dx = b[0] - a[0], dy = b[1] - a[1], l = Math.hypot(dx, dy);
    return [p[0] - (dy / l) * k * sgn, p[1] + (dx / l) * k * sgn] as [number, number];
  });
  return poly([...side(1), ...side(-1).reverse()]);
}
/** A plus as one outline (so evenodd fill leaves no hole where the bars cross), optionally turned. */
function cross(cx: number, cy: number, a: number, k: number, turn = 0) {
  const pts: [number, number][] = [[-k, -a], [k, -a], [k, -k], [a, -k], [a, k], [k, k], [k, a], [-k, a], [-k, k], [-a, k], [-a, -k], [-k, -k]];
  const c = Math.cos(turn), s = Math.sin(turn);
  return poly(pts.map(([x, y]) => [cx + x * c - y * s, cy + x * s + y * c]));
}

export function eyePath(side: "L" | "R", e: Eye): string {
  const w = W * (e.sx ?? 1), h = H * (e.sy ?? 1);
  const cx = CX[side] + (e.dx ?? 0) * W, cy = CY + (e.dy ?? 0) * H;
  const x0 = cx - w / 2, x1 = cx + w / 2, y0 = cy - h / 2, y1 = cy + h / 2;
  // outer corner is the left one on the left eye
  const [lo, li] = e.lid ?? [0, 0], [bo, bi] = e.base ?? [0, 0];
  const [tl, tr] = side === "L" ? [lo, li] : [li, lo];
  const [bl, br] = side === "L" ? [bo, bi] : [bi, bo];
  const t = 16; // stroke-ish thickness for line shapes
  switch (e.kind ?? "block") {
    case "block": return poly([[x0, y0 + tl * h], [x1, y0 + tr * h], [x1, y1 - br * h], [x0, y1 - bl * h]]);
    case "happy": return poly([[x0, cy + 14], [cx, cy - 26], [x1, cy + 14], [x1, cy + 14 + t * 1.6], [cx, cy - 26 + t * 1.8], [x0, cy + 14 + t * 1.6]]);
    case "sad-arc": return poly([[x0, cy - 14], [cx, cy + 26], [x1, cy - 14], [x1, cy - 14 - t * 1.6], [cx, cy + 26 - t * 1.8], [x0, cy - 14 - t * 1.6]]);
    case "line": return poly([[x0, cy - t / 2], [x1, cy - t / 2], [x1, cy + t / 2], [x0, cy + t / 2]]);
    case "dot": { const d = 15 * (e.sx ?? 1); return rect(cx - d, cy - d, cx + d, cy + d); }
    case "plus": return cross(cx, cy, 26, 8);
    case "minus": return rect(cx - 26, cy - 8, cx + 26, cy + 8);
    case "divide": return rect(cx - 26, cy - 7, cx + 26, cy + 7) + rect(cx - 8, cy - 33, cx + 8, cy - 17) + rect(cx - 8, cy + 17, cx + 8, cy + 33);
    case "equals": return rect(cx - 26, cy - 20, cx + 26, cy - 6) + rect(cx - 26, cy + 6, cx + 26, cy + 20);
    case "percent": return rect(cx - 26, cy - 26, cx - 10, cy - 10) + rect(cx + 10, cy + 10, cx + 26, cy + 26) + stroke([[cx - 24, cy + 28], [cx + 24, cy - 28]], 6);
    case "sqrt": return stroke([[cx - 30, cy + 2], [cx - 18, cy + 2], [cx - 6, cy + 28], [cx + 8, cy - 26], [cx + 30, cy - 26]], 6);
    case "approx": return stroke([[cx - 28, cy - 8], [cx - 12, cy - 18], [cx + 12, cy - 6], [cx + 28, cy - 16]], 6) + stroke([[cx - 28, cy + 16], [cx - 12, cy + 6], [cx + 12, cy + 18], [cx + 28, cy + 8]], 6);
    case "lt": return stroke([[cx + 22, cy - 28], [cx - 22, cy], [cx + 22, cy + 28]], 7);
    case "gt": return stroke([[cx - 22, cy - 28], [cx + 22, cy], [cx - 22, cy + 28]], 7);
    case "plusminus": return cross(cx, cy - 10, 20, 7) + rect(cx - 22, cy + 20, cx + 22, cy + 34);
    case "pi": return rect(cx - 30, cy - 26, cx + 30, cy - 12) + rect(cx - 18, cy - 12, cx - 4, cy + 28) + rect(cx + 6, cy - 12, cx + 20, cy + 28);
    case "power": return stroke([[cx - 26, cy + 26], [cx - 10, cy - 8], [cx + 6, cy + 26]], 6) + rect(cx + 14, cy - 30, cx + 30, cy - 14);
    case "wave": {
      // blocky waveform: five chunky bars per side spanning the whole eye row, heights snapped to 12-unit steps
      const from = side === "L" ? 151 : MIDX + 3, ph = e.beam ?? 0;
      let d = "";
      for (let i = 0; i < 5; i++) {
        const x = from + i * 21, gx = x - 150;
        const amp = Math.abs(Math.sin(gx * 0.035 + ph * 2.1) * 0.6 + Math.sin(gx * 0.08 - ph * 3.3) * 0.4);
        const bh = Math.max(12, Math.round((amp * 92) / 12) * 12);
        d += rect(x, CY - bh / 2, x + 15, CY + bh / 2);
      }
      return d;
    }
    case "sync": {
      // the eye's outline as tiny boxes; a run of bigger boxes chases round it
      const N = 22, per = 2 * (w + h), run = 5, head = (e.beam ?? 0) * N;
      let d = "";
      for (let i = 0; i < N; i++) {
        let u = (i / N) * per;
        const [px, py] = u < w ? [x0 + u, y0] : (u -= w) < h ? [x1, y0 + u] : (u -= h) < w ? [x1 - u, y1] : [x0, y1 - (u - w)];
        const lag = (head - i + N) % N, s2 = lag < run ? 7 - lag * 0.6 : 2.6;
        d += rect(px - s2, py - s2, px + s2, py + s2);
      }
      return d;
    }
    case "radar": {
      // a square scope: thin frame, a sweep arm turning from the centre out to the frame, a hub
      const s = 6, half = Math.min(w, h) / 2, a = (e.beam ?? 0) * Math.PI * 2, reach = (half - s - 2) / Math.max(Math.abs(Math.cos(a)), Math.abs(Math.sin(a)));
      return rect(cx - half, cy - half, cx + half, cy + half) + rect(cx - half + s, cy - half + s, cx + half - s, cy + half - s)
        + stroke([[cx, cy], [cx + Math.cos(a) * reach, cy + Math.sin(a) * reach]], 4) + rect(cx - 5, cy - 5, cx + 5, cy + 5);
    }
    // One big square radar across both eyes (drawn from the left eye; the right eye hides). beam is the sweep, 0..1.
    case "scope": case "scope-grid": case "scope-wedge": {
      const S2 = SCOPE / 2, mx = MIDX, my = CY, a = (e.beam ?? 0) * Math.PI * 2 - Math.PI / 2, R = S2 - 12;
      const circle = (r: number) => poly(Array.from({ length: 48 }, (_, i) => [mx + Math.cos((i / 48) * Math.PI * 2) * r, my + Math.sin((i / 48) * Math.PI * 2) * r] as [number, number]));
      if (e.kind === "scope-grid") {
        // rings and a crosshair, in the darker shade
        return [R, R * 0.68, R * 0.36].map((r) => circle(r) + circle(r - 2.5)).join("")
          + rect(mx - R, my - 1.2, mx - 6, my + 1.2) + rect(mx + 6, my - 1.2, mx + R, my + 1.2) + rect(mx - 1.2, my - R, mx + 1.2, my - 6) + rect(mx - 1.2, my + 6, mx + 1.2, my + R);
      }
      if (e.kind === "scope-wedge") {
        // the wake behind the arm
        const pts: [number, number][] = [[mx, my]];
        for (let i = 0; i <= 12; i++) { const b = a - (i / 12) * 0.9; pts.push([mx + Math.cos(b) * R, my + Math.sin(b) * R]); }
        return poly(pts);
      }
      // lit: frame, rim ticks, arm, hub, and the blips the arm has just passed
      const f = 5, ticks = rect(mx - 2, my - S2 + f + 3, mx + 2, my - S2 + f + 11) + rect(mx - 2, my + S2 - f - 11, mx + 2, my + S2 - f - 3)
        + rect(mx - S2 + f + 3, my - 2, mx - S2 + f + 11, my + 2) + rect(mx + S2 - f - 11, my - 2, mx + S2 - f - 3, my + 2);
      let d = rect(mx - S2, my - S2, mx + S2, my + S2) + rect(mx - S2 + f, my - S2 + f, mx + S2 - f, my + S2 - f) + ticks
        + stroke([[mx + Math.cos(a) * 9, my + Math.sin(a) * 9], [mx + Math.cos(a) * R, my + Math.sin(a) * R]], 2.5) + rect(mx - 6, my - 6, mx + 6, my + 6);
      for (const [deg, k] of BLIPS) {
        const b = (deg / 180) * Math.PI - Math.PI / 2, behind = ((a - b) % (Math.PI * 2) + Math.PI * 2) % (Math.PI * 2);
        const sz = behind < 1.4 ? 4.5 : 2.2, bx = mx + Math.cos(b) * R * k, by = my + Math.sin(b) * R * k;
        d += rect(bx - sz, by - sz, bx + sz, by + sz);
      }
      return d;
    }
    case "bang": return rect(cx - 9, cy - 44, cx + 9, cy + 14) + rect(cx - 9, cy + 26, cx + 9, cy + 44);
    case "slices": {
      // glitch: the block cut into bands, each knocked sideways
      const off = e.slices ?? [0, 0, 0, 0], bh = h / off.length;
      return off.map((o, i) => rect(x0 + o * W, y0 + i * bh, x1 + o * W, y0 + (i + 1) * bh + (i < off.length - 1 ? 0.5 : 0))).join("");
    }
    case "scan": {
      // a lit eye with a dark scanline sweeping through it, a fainter line trailing behind (cut out with evenodd)
      const by = y0 + 6 + (h - 20) * (e.beam ?? 0.5), trail = (e.lid?.[0] ?? 0) >= 0 ? -16 : 16;
      const ty = Math.min(y1 - 6, Math.max(y0 + 4, by + trail));
      return rect(x0, y0, x1, y1) + rect(x0, by, x1, by + 8) + rect(x0, ty, x1, ty + 3);
    }
    case "ring": {
      const s = 16;
      return poly([[x0, y0], [x1, y0], [x1, y1], [x0, y1]]) + poly([[x0 + s, y0 + s], [x0 + s, y1 - s], [x1 - s, y1 - s], [x1 - s, y0 + s]]);
    }
    case "x": return cross(cx, cy, w * 0.62, 8, Math.PI / 4);
    case "heart": {
      const a = w * 0.62;
      return poly([[cx, cy - a * 0.35], [cx - a * 0.5, cy - a * 0.85], [cx - a, cy - a * 0.35], [cx - a, cy + a * 0.05], [cx, cy + a], [cx + a, cy + a * 0.05], [cx + a, cy - a * 0.35], [cx + a * 0.5, cy - a * 0.85]]);
    }
    case "star": {
      const a = h * 0.42, b = 9;
      return poly([[cx, cy - a], [cx + b, cy - b], [cx + a * 0.75, cy], [cx + b, cy + b], [cx, cy + a], [cx - b, cy + b], [cx - a * 0.75, cy], [cx - b, cy - b]]);
    }
    case "chev": {
      const a = w * 0.42;
      return poly([[cx - a, cy - a * 1.2], [cx - a + t * 1.4, cy - a * 1.2], [cx + a, cy], [cx - a + t * 1.4, cy + a * 1.2], [cx - a, cy + a * 1.2], [cx + a - t * 1.4, cy]]);
    }
  }
}

const both = (e: Eye): Face => ({ L: e, R: e });

export const MOODS = ["idle", "happy", "laughing", "love", "wink", "surprised", "thinking", "searching", "confused", "nervous", "sad", "angry", "sleeping", "celebrate", "serious", "calculating", "puzzling", "hearing", "posting", "catching", "syncing", "waiting", "ringing", "low-battery", "scanning", "orbiting", "voice", "bored", "loading", "error", "glitch", "shy"] as const;
export type Mood = (typeof MOODS)[number];
export const moodLabel = (m: Mood) => (m === "low-battery" ? "Low battery" : m[0].toUpperCase() + m.slice(1));

export const FACES: Record<Mood, Face> = {
  idle: both({}),
  happy: both({ kind: "happy" }),
  laughing: both({ kind: "happy", sx: 1.15, dy: -0.06 }),
  love: both({ kind: "heart" }),
  wink: { L: {}, R: { sy: 0.14 } },
  surprised: both({ kind: "ring" }),
  thinking: both({ sy: 0.62, dy: -0.19, dx: 0.12 }),
  searching: both({ sy: 0.7, dx: 0.26 }),
  confused: { L: {}, R: { sy: 0.5, dy: 0.1, lid: [0.2, 0] } },
  nervous: both({ sy: 0.62, lid: [0, 0.25], dx: -0.08 }),
  sad: both({ lid: [0.5, 0] }),
  angry: both({ lid: [0, 0.5] }),
  sleeping: both({ kind: "line", dy: 0.24 }),
  celebrate: both({ kind: "star" }),
  serious: both({ lid: [0.38, 0.38] }),
  calculating: { L: { kind: "plus" }, R: { kind: "equals" } },
  puzzling: { L: { lid: [0.32, 0] }, R: { sy: 0.55, dy: 0.18 } },
  hearing: both({ sy: 0.8, dx: -0.28 }),
  posting: both({ kind: "happy", dy: -0.27, ghost: [{ kind: "happy", dy: 0.2 }] }),
  catching: both({ kind: "ring", sx: 0.85, sy: 0.85, dy: -0.14 }),
  syncing: { L: { kind: "sync", beam: 0.2 }, R: { kind: "sync", beam: 0.7 } },
  waiting: both({ kind: "line" }),
  ringing: { L: { kind: "ring", sx: 0.8, sy: 0.8 }, R: { kind: "ring", sx: 0.8, sy: 0.8, dy: -0.1 } },
  "low-battery": both({ lid: [0.68, 0.68] }),
  scanning: both({ kind: "scan", beam: 0.35 }),
  voice: both({ kind: "wave", beam: 0.6 }),
  loading: { L: { lid: [0.45, 0.45], ghost: [{}] }, R: { lid: [1, 1], ghost: [{}] } },
  error: both({ kind: "bang" }),
  glitch: both({ kind: "slices", slices: [0.12, -0.08, 0.18, -0.04] }),
  shy: both({ sy: 0.5, dy: 0.24, dx: -0.2, lid: [0.2, 0.2] }),
  bored: { L: { lid: [0.5, 0.5], dx: 0.18, dy: 0.06 }, R: { lid: [0.5, 0.5], dx: 0.18, dy: 0.06 } },
  orbiting: { L: { kind: "dot", dx: -0.5, dy: -0.2 }, R: { kind: "dot", dx: 0.5, dy: 0.2 } },
};

// ── Animated moods: a face per moment t (seconds). Moods not listed here hold still.
const mix = (a = 0, b = 0, k: number) => a + (b - a) * k;
const ease = (k: number) => k * k * (3 - 2 * k);
function lerpEye(a: Eye, b: Eye, k: number): Eye {
  return {
    kind: k < 0.5 ? a.kind : b.kind,
    dx: mix(a.dx, b.dx, k), dy: mix(a.dy, b.dy, k), sx: mix(a.sx ?? 1, b.sx ?? 1, k), sy: mix(a.sy ?? 1, b.sy ?? 1, k),
    lid: [mix(a.lid?.[0], b.lid?.[0], k), mix(a.lid?.[1], b.lid?.[1], k)],
    base: [mix(a.base?.[0], b.base?.[0], k), mix(a.base?.[1], b.base?.[1], k)],
  };
}
/** Hold each key for `hold` seconds, then ease into the next over `move` seconds. */
function keys(list: Face[], t: number, hold: number, move: number): Face {
  t = Math.max(0, t);
  const span = hold + move, n = list.length, i = Math.floor(t / span) % n, into = (t % span) - hold;
  if (into <= 0) return list[i];
  const k = ease(into / move), a = list[i], b = list[(i + 1) % n];
  return { L: lerpEye(a.L, b.L, k), R: lerpEye(a.R, b.R, k) };
}
const MID = (CX.L + CX.R) / 2;
const SYMBOLS: EyeKind[] = ["plus", "minus", "x", "divide", "equals", "percent", "sqrt", "approx", "lt", "gt", "plusminus", "pi", "power"];
const THINK: Face[] = [
  both({ sy: 0.62, dy: -0.19, dx: -0.2 }),
  both({ sy: 0.62, dy: -0.19, dx: 0.2 }),
  both({ sy: 0.6, dy: 0.2, dx: -0.15 }),
  FACES.confused,
];

const BORED: Face[] = [
  both({ lid: [0.5, 0.5], dx: 0.18, dy: 0.06 }),
  both({ lid: [0.5, 0.5], dx: -0.18, dy: 0.06 }),
  both({ lid: [0.72, 0.72], dx: -0.05, dy: 0.1 }),
  both({ lid: [0.5, 0.5], dx: 0.05, dy: 0.06 }),
];

const SEEK: Face[] = [
  both({ sy: 0.7, dy: -0.2 }), both({ sy: 0.7, dy: 0.2 }), both({ sy: 0.7, dx: -0.28 }), both({ sy: 0.7, dx: 0.28 }),
];
const SHY: Face[] = [
  both({ sy: 0.5, dy: 0.24, dx: -0.2, lid: [0.2, 0.2] }),
  both({ sy: 0.5, dy: 0.24, dx: -0.2, lid: [0.2, 0.2] }),
  both({ sy: 0.62, dy: -0.04, dx: 0.12 }),
  both({ sy: 0.55, dy: 0.22, dx: -0.24, lid: [0.12, 0.12] }),
];

/** Two moons on one ring, half a turn apart; the near one swells, the far one shrinks. */
function orbit(t: number): Face {
  const at = (phase: number, side: "L" | "R"): Eye => {
    const a = t * 1.6 + phase;
    return { kind: "dot", dx: (MID + Math.cos(a) * 58 - CX[side]) / W, dy: (Math.sin(a) * 30) / H, sx: 1 + Math.sin(a) * 0.3 };
  };
  return { L: at(Math.PI, "L"), R: at(0, "R") };
}

export const ANIMATED = new Set<Mood>(["wink", "ringing", "loading", "error", "glitch", "shy", "laughing", "posting", "syncing", "orbiting", "calculating", "searching", "scanning", "thinking", "voice", "bored"]);
export function faceAt(mood: Mood, t: number): Face {
  switch (mood) {
    case "orbiting": return orbit(t);
    case "laughing": {
      // both eyes bob together, like a belly laugh
      return both({ kind: "happy", sx: 1.15, dy: 0.02 - 0.17 * Math.abs(Math.sin(t * 9)) });
    }
    case "posting": {
      // an upload: two chevrons stacked; the lit one climbs from the lower to the upper, the unlit one is a darker shade
      const lo: Eye = { kind: "happy", dy: 0.2 }, hi: Eye = { kind: "happy", dy: -0.27 };
      const k = Math.floor(t / 0.35) % 3;
      return both(k === 0 ? { ...lo, ghost: [hi] } : k === 1 ? { ...hi, ghost: [lo] } : { kind: "dot", sx: 0, ghost: [lo, hi] });
    }
    case "syncing": {
      const b = (t * 1.1) % 1;
      return { L: { kind: "sync", beam: b }, R: { kind: "sync", beam: (b + 0.5) % 1 } };
    }
    case "calculating": {
      // one eye at a time: left, right, left, right…
      const k = Math.floor(t / 0.5), n = SYMBOLS.length;
      return { L: { kind: SYMBOLS[(2 * Math.floor((k + 1) / 2)) % n] }, R: { kind: SYMBOLS[(2 * Math.floor(k / 2) + 1) % n] } };
    }
    case "searching": {
      // glance up, down, left, right, then square off into radar scopes that sweep, and back
      const looks = SEEK.length * 0.75 - 0.3, morph = 0.45, sweep = 2.8, cycle = looks + morph * 2 + sweep, u = t % cycle;
      if (u < looks) return keys(SEEK, u, 0.45, 0.3);
      // the two eyes slide together and grow into one square, which becomes the radar
      const last = SEEK[SEEK.length - 1], into = (MIDX - CX.L) / W;
      const merged: Face = { L: { dx: into, sx: SCOPE / W, sy: SCOPE / H }, R: { dx: -into, sx: SCOPE / W, sy: SCOPE / H } };
      const mix2 = (a: Face, b: Face, k: number): Face => ({ L: lerpEye(a.L, b.L, k), R: lerpEye(a.R, b.R, k) });
      if (u < looks + morph) return mix2(last, merged, ease((u - looks) / morph));
      if (u < looks + morph + sweep) {
        const beam = ((u - looks - morph) / 1.6) % 1;
        return { L: { kind: "scope", beam, ghost: [{ kind: "scope-grid" }, { kind: "scope-wedge", beam }] }, R: { kind: "dot", sx: 0 } };
      }
      return mix2(merged, SEEK[0], ease((u - looks - morph - sweep) / morph));
    }
    case "ringing": {
      // rings shake in bursts, like a phone going off, then pause
      const k = t % 1.4, on = k < 0.8, jig = on ? (Math.floor(k / 0.05) % 2 ? 0.07 : -0.07) : 0;
      const s = on ? 0.85 + 0.06 * Math.sin(k * 40) : 0.8;
      return { L: { kind: "ring", sx: s, sy: s, dx: jig }, R: { kind: "ring", sx: s, sy: s, dx: jig, dy: -0.1 } };
    }
    case "loading": {
      // each eye is a track in a darker shade; the left fills from the bottom, then the right, then both empty
      const k = (t % 3) / 3, l = Math.min(1, k / 0.45), r = Math.max(0, Math.min(1, (k - 0.45) / 0.45));
      const fill = (p: number): Eye => ({ lid: [1 - Math.floor(p * 6) / 6, 1 - Math.floor(p * 6) / 6], ghost: [{}] });
      return { L: fill(k > 0.95 ? 0 : l), R: fill(k > 0.95 ? 0 : r) };
    }
    case "error": {
      // "!" eyes flash between lit and the darker shade, jolting sideways on each flash
      const k = Math.floor(t / 0.22), on = k % 4 !== 3 && k % 2 === 0, jolt = on ? (k % 8 < 4 ? 0.05 : -0.05) : 0;
      return on ? both({ kind: "bang", dx: jolt }) : both({ kind: "dot", sx: 0, ghost: [{ kind: "bang" }] });
    }
    case "glitch": {
      // hold still, then judder: the bands jump to new offsets every 80ms for a moment
      const k = t % 1.6;
      if (k > 0.5) return both({});
      const n = Math.floor(t / 0.08), rnd = (i: number) => ((Math.sin(n * 12.9898 + i * 78.233) * 43758.5453) % 1);
      return { L: { kind: "slices", slices: [0, 1, 2, 3].map((i) => rnd(i) * 0.25) }, R: { kind: "slices", slices: [4, 5, 6, 7].map((i) => rnd(i) * 0.25) } };
    }
    case "shy": return keys(SHY, t, 1.2, 0.5);
    case "wink": {
      // open, squash down to a thin bar, hold the wink, open again
      const k = t % 2.4, open = 1 - Math.min(1, Math.max(0, (k - 0.9) / 0.12)) + Math.min(1, Math.max(0, (k - 1.75) / 0.12));
      return { L: {}, R: { sy: Math.max(0.14, Math.min(1, open)) } };
    }
    case "scanning": {
      // sweep down then back up; the trail follows on the side the beam came from
      const ph = t * 2.2, beam = (1 - Math.cos(ph)) / 2, down = Math.sin(ph) >= 0;
      return both({ kind: "scan", beam, lid: [down ? 0 : -1, 0] });
    }
    case "thinking": {
      // look around, puzzle, then shrink into two moons that orbit for a while, and grow back
      const looks = THINK.length * 1.35 - 0.45, morph = 0.5, spin = 2.6, cycle = looks + morph * 2 + spin;
      const u = t % cycle;
      if (u < looks) return keys(THINK, u, 0.9, 0.45);
      const o = orbit(u - looks), moon = (e: Eye): Eye => ({ dx: e.dx, dy: e.dy, sx: (30 / W) * (e.sx ?? 1), sy: (30 / H) * (e.sx ?? 1) });
      if (u < looks + morph) { const k = ease((u - looks) / morph); return { L: lerpEye(FACES.confused.L, moon(o.L), k), R: lerpEye(FACES.confused.R, moon(o.R), k) }; }
      if (u < looks + morph + spin) return o;
      const end = orbit(spin), k = ease((u - looks - morph - spin) / morph);
      return { L: lerpEye(moon(end.L), THINK[0].L, k), R: lerpEye(moon(end.R), THINK[0].R, k) };
    }
    // steps of 0.12s keep the bars blocky in time too
    case "voice": return both({ kind: "wave", beam: Math.floor(t / 0.12) * 0.12 });
    // slow sideways glances and a heavy-lidded sag
    case "bored": return keys(BORED, t, 1.8, 1.1);
    default: return FACES[mood];
  }
}

/** Arch stops (start, middle, end, as in the original mark) and the eye colour. */
export interface Preset { id: string; name: string; arch: string; eyes: string }
export const PRESETS: Preset[] = [
  { id: "astrid", name: "Astrid", arch: "4459F9, 1A32EC, 4459F9", eyes: "00F7D5" },
  { id: "cherry", name: "Cherry", arch: "F94447, EC1A1D, F94447", eyes: "F7C600" },
  { id: "moss", name: "Moss", arch: "5FBF5A, 2E8B3A, 5FBF5A", eyes: "F2F7A1" },
  { id: "grape", name: "Grape", arch: "9B6BFF, 6A2BEA, 9B6BFF", eyes: "FFB3E6" },
  { id: "sun", name: "Sun", arch: "FFC93D, F59E0B, FFC93D", eyes: "FFF8DC" },
  { id: "ember", name: "Ember", arch: "FF8A3D, E8551A, FF8A3D", eyes: "FFF1D6" },
  { id: "mint", name: "Mint", arch: "3EE6C1, 00B894, 3EE6C1", eyes: "EFFFFA" },
  { id: "mist", name: "Mist", arch: "B8C0FF, 8793FF, B8C0FF", eyes: "1A32EC" },
  { id: "ink", name: "Ink", arch: "3A3A3A, 1A1A1A, 3A3A3A", eyes: "FF5A1F" },
  { id: "paper", name: "Paper", arch: "EEEEEE, CFCFCF, EEEEEE", eyes: "1A32EC" },
  { id: "bubblegum", name: "Bubblegum", arch: "FF8FC8, F0479A, FF8FC8", eyes: "FFF0F7" },
  { id: "ocean", name: "Ocean", arch: "4FC3F7, 0277BD, 4FC3F7", eyes: "FFE066" },
  { id: "gold", name: "Gold", arch: "F2D06B, C99A2E, F2D06B", eyes: "1A1A1A" },
  { id: "lilac", name: "Lilac", arch: "C9B6FF, 9B7BFF, C9B6FF", eyes: "2A1A5E" },
  { id: "slate", name: "Slate", arch: "9AA5B1, 5B6774, 9AA5B1", eyes: "7CFFB2" },
];

/** "F94447, EC1A1D, F94447" → ["#F94447", …]. One to three stops; null if any stop isn't a hex colour. */
export function parseStops(text: string): string[] | null {
  const parts = text.split(/[\s,]+/).filter(Boolean).map((p) => p.replace(/^#/, "").toUpperCase());
  if (!parts.length || parts.length > 3) return null;
  const full = parts.map((p) => (/^[0-9A-F]{3}$/.test(p) ? p.replace(/./g, "$&$&") : p));
  return full.every((p) => /^[0-9A-F]{6}$/.test(p)) ? full.map((p) => "#" + p) : null;
}
export const formatStops = (stops: string[]) => stops.map((s) => s.replace("#", "").toUpperCase()).join(", ");
