// Vendored from github.com/404khai/goblins (9e63cf1, MIT, Copyright (c) 2026 Khai; see web/src/goblin/LICENSE).
import { EXPRESSIONS, STILLS } from "./expressions";
import type { ExpressionId, Goblin } from "./types";

/**
 * Flat, front-facing SVG of a goblin. Shares the trait set and expression poses with the 3D rig,
 * so a goblin designed in the studio exports as a matching sticker.
 * 1 rig unit = 60px; the head sits at (120, 150) in a 320×320 viewBox.
 */

const INK = "#1D2318";
const R = 60;
const X = (x: number) => +(120 + x * R).toFixed(2);
const Y = (y: number) => +(150 - y * R).toFixed(2);
const SW = 4; // outline width
const HEADS: Record<Goblin["head"], [number, number]> = { round: [1.04, 0.96], egg: [0.9, 1.1], wide: [1.2, 0.86], bean: [0.98, 1.0] };

const shade = (hex: string, f: number) => {
  const n = parseInt(hex.slice(1), 16);
  const c = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => Math.round(Math.min(255, v * f)));
  return "#" + c.map((v) => v.toString(16).padStart(2, "0")).join("");
};
const deg = (rad: number) => +((rad * 180) / Math.PI).toFixed(2);
const attrs = (a: Record<string, string | number | undefined>) =>
  Object.entries(a).filter(([, v]) => v !== undefined).map(([k, v]) => `${k}="${v}"`).join(" ");
const el = (tag: string, a: Record<string, string | number | undefined>, inner?: string) =>
  inner === undefined ? `<${tag} ${attrs(a)}/>` : `<${tag} ${attrs(a)}>${inner}</${tag}>`;
const line = { stroke: INK, "stroke-width": SW, "stroke-linejoin": "round", "stroke-linecap": "round" };

export interface SvgOptions {
  expression?: ExpressionId;
  /** Pixel width/height attributes. Omit for a fluid SVG. */
  size?: number;
  /** Allow the floating mitts. They only appear on expressions that use them (thinking, searching, confused). Default true. */
  hands?: boolean;
  /** Draw expression props such as Z's, hearts or the magnifier. Default true. */
  props?: boolean;
  /** Background fill; transparent when omitted. */
  background?: string;
  /** Prefix for ids inside the SVG so several can share one page. */
  idPrefix?: string;
}

export function goblinSvg(d: Goblin, opts: SvgOptions = {}): string {
  const id = opts.idPrefix ?? `gb${Math.random().toString(36).slice(2, 7)}`;
  const exprId = opts.expression ?? d.expression ?? "idle";
  const ex = EXPRESSIONS[exprId] ?? EXPRESSIONS.idle;
  const pose = ex.pose;
  const still = STILLS[exprId] ?? {};
  const [hx, hy] = HEADS[d.head] ?? HEADS.round;
  const skin = d.skin, dark = shade(d.skin, 0.8);
  const hasBeard = d.beard !== "none";
  const out: string[] = [];
  const back: string[] = [];
  const front: string[] = [];

  const ell = (cx: number, cy: number, rx: number, ry: number, fill: string, extra: Record<string, string | number | undefined> = {}) =>
    el("ellipse", { cx: X(cx), cy: Y(cy), rx: +(rx * R).toFixed(2), ry: +(ry * R).toFixed(2), fill, ...line, ...extra });
  const circ = (cx: number, cy: number, r: number, fill: string, extra: Record<string, string | number | undefined> = {}) =>
    el("circle", { cx: X(cx), cy: Y(cy), r: +(r * R).toFixed(2), fill, ...line, ...extra });
  const path = (dd: string, fill: string, extra: Record<string, string | number | undefined> = {}) => el("path", { d: dd, fill, ...line, ...extra });

  // ---------- ears (behind the head) ----------
  const EARS = { pointy: [0.24, 0.9, -0.15], long: [0.2, 1.4, 0.35], droopy: [0.24, 0.95, -0.85], tiny: [0.17, 0.5, 0.05] } as const;
  for (const s of [-1, 1]) {
    if (d.ears === "round") {
      back.push(ell(s * hx * 0.97, 0.12, 0.2, 0.3, skin), ell(s * hx * 0.99, 0.12, 0.1, 0.17, dark, { stroke: "none" }));
      continue;
    }
    const [r, h, a] = EARS[d.ears] ?? EARS.pointy;
    const dx = s * Math.cos(a), dy = Math.sin(a);
    const bx = s * hx * 0.78, by = 0.08;
    const tip = [bx + dx * h, by + dy * h];
    const px = -dy, py = dx; // perpendicular
    const tri = (rr: number, len: number) =>
      `M${X(bx + px * rr)} ${Y(by + py * rr)} Q${X(bx + dx * len * 0.55 + px * rr * 0.9)} ${Y(by + dy * len * 0.55 + py * rr * 0.9)} ${X(bx + dx * len)} ${Y(by + dy * len)} Q${X(bx + dx * len * 0.55 - px * rr * 0.9)} ${Y(by + dy * len * 0.55 - py * rr * 0.9)} ${X(bx - px * rr)} ${Y(by - py * rr)}Z`;
    back.push(path(tri(r, h), skin), path(tri(r * 0.5, h * 0.7), dark, { stroke: "none" }));
    void tip;
  }

  // ---------- head with a flat toon shade ----------
  const head = `${id}-head`;
  out.push(
    el("clipPath", { id: head }, el("ellipse", { cx: X(0), cy: Y(0), rx: hx * R, ry: hy * R })),
    el("ellipse", { cx: X(0), cy: Y(0), rx: hx * R, ry: hy * R, fill: dark }),
    el("ellipse", { cx: X(-0.1), cy: Y(0.1), rx: hx * R * 0.97, ry: hy * R * 0.95, fill: skin, "clip-path": `url(#${head})` }),
    el("ellipse", { cx: X(0), cy: Y(0), rx: hx * R, ry: hy * R, fill: "none", ...line }),
  );
  if (d.extras.includes("freckles"))
    for (const s of [-1, 1]) for (const [fx, fy] of [[0.5, -0.12], [0.62, -0.2], [0.48, -0.26]]) out.push(circ(s * fx * hx, fy, 0.035, dark, { stroke: "none" }));

  // ---------- scarf ----------
  if (d.scarf !== "none") {
    const sc = d.scarfColor, y = -0.86 * hy;
    if (d.scarf === "long") out.push(el("rect", { x: X(0.43), y: Y(-1.0), width: 0.38 * R, height: 1.0 * R, rx: 5, fill: sc, ...line, transform: `rotate(-10 ${X(0.62)} ${Y(-1.35)})` }) +
      el("rect", { x: X(0.43), y: Y(-1.9), width: 0.38 * R, height: 0.12 * R, fill: shade(sc, 0.75), ...line, "stroke-width": 3, transform: `rotate(-10 ${X(0.62)} ${Y(-1.35)})` }));
    out.push(el("rect", { x: X(-0.96 * hx), y: Y(y + 0.24), width: 1.92 * hx * R, height: 0.48 * R, rx: 0.24 * R, fill: sc, ...line }));
    out.push(el("line", { x1: X(-0.8 * hx), x2: X(0.8 * hx), y1: Y(y), y2: Y(y), stroke: shade(sc, 0.8), "stroke-width": 3, "stroke-linecap": "round" }));
    if (d.scarf === "wrap") out.push(ell(0.45, -0.95, 0.22, 0.18, sc));
  }

  // ---------- beard ----------
  const bc = d.beardColor;
  const blob = (parts: string[]) => {
    // draw outlines first, then fills on top, so overlapping shapes read as one mass
    out.push(...parts.map((p) => p.replace(/fill="[^"]*"/, `fill="${INK}"`).replace(/stroke-width="4"/, `stroke-width="${SW * 2}"`)));
    out.push(...parts.map((p) => p.replace(/stroke="#1D2318"/, 'stroke="none"')));
  };
  if (d.beard === "full") blob([ell(0, -0.62, 0.9 * 1.05 * hx, 0.9 * 1.05, bc), circ(0, -1.25, 0.5, bc)]);
  else if (d.beard === "pointy") blob([path(`M${X(-0.72 * hx)} ${Y(-0.35)} Q${X(-0.5 * hx)} ${Y(-1.2)} ${X(0)} ${Y(-1.95)} Q${X(0.5 * hx)} ${Y(-1.2)} ${X(0.72 * hx)} ${Y(-0.35)} Q${X(0)} ${Y(-0.15)} ${X(-0.72 * hx)} ${Y(-0.35)}Z`, bc)]);
  else if (d.beard === "forked") blob([
    ell(0, -0.6, 0.77 * hx, 0.56, bc),
    path(`M${X(-0.55)} ${Y(-0.75)} L${X(-0.42)} ${Y(-1.75)} L${X(-0.05)} ${Y(-0.85)}Z`, bc),
    path(`M${X(0.55)} ${Y(-0.75)} L${X(0.42)} ${Y(-1.75)} L${X(0.05)} ${Y(-0.85)}Z`, bc),
  ]);
  else if (d.beard === "braid") {
    blob([ell(0, -0.55, 0.77 * hx, 0.52, bc), ...[0.3, 0.25, 0.21, 0.17].map((r, k) => ell(0, -1.0 - k * 0.32, r, r * 0.9, bc))]);
    out.push(circ(0, -2.08, 0.1, "#D8402F"));
  }
  // ---------- eyes ----------
  const EX = hx * 0.36, EY = 0.15;
  const look = still.look ?? [0, 0];
  const lid = pose.lid, wide = 1 + Math.max(0, -lid) * 1.2;
  const visor = d.eyes === "visor";
  const GLOW = "#7CF2C8";
  if (visor) out.push(el("rect", { x: X(-hx * 0.6), y: Y(EY + 0.22), width: hx * 1.2 * R, height: 0.44 * R, rx: 12, fill: "#151515", ...line }));
  [-1, 1].forEach((s, i) => {
    const mode = pose.eyes[i];
    const cx = s * EX, cy = EY;
    const arcColor = visor ? GLOW : INK;
    if (mode !== "open") {
      const up = mode === "happy" ? 1 : -1;
      out.push(path(`M${X(cx - 0.14)} ${Y(cy - 0.03 * up)} Q${X(cx)} ${Y(cy + 0.2 * up)} ${X(cx + 0.14)} ${Y(cy - 0.03 * up)}`, "none", { stroke: arcColor, "stroke-width": 5 }));
      return;
    }
    if (visor) {
      const h = 0.13 * (1 - Math.max(0, lid) * 0.6);
      out.push(el("rect", { x: X(cx - 0.12 + look[0] * 0.07), y: Y(cy + h / 2 + look[1] * 0.07), width: 0.24 * R, height: h * R, rx: 3, fill: GLOW, transform: `rotate(${-deg(s * pose.tilt * 0.6)} ${X(cx)} ${Y(cy)})` }));
      return;
    }
    if (d.eyes === "beady") {
      out.push(ell(cx + look[0] * 0.03, cy + look[1] * 0.03, 0.12 * wide, 0.14 * wide, INK, { stroke: "none" }));
      return;
    }
    const big = d.eyes === "wonky" && s < 0 ? 1.25 : 1;
    const box = d.eyes === "wonky" && s > 0;
    const [sx, sy] = d.eyes === "oval" ? [0.85, 1.4] : [1, 1.12];
    const rx = 0.22 * big * sx * wide, ry = 0.22 * big * sy * wide;
    const clip = `${id}-eye${i}`;
    const shape = box
      ? { tag: "rect", a: { x: X(cx - 0.18), y: Y(cy + 0.12), width: 0.44 * R, height: 0.32 * R, rx: 7, transform: `rotate(7 ${X(cx)} ${Y(cy)})` } }
      : { tag: "ellipse", a: { cx: X(cx), cy: Y(cy), rx: rx * R, ry: ry * R } };
    out.push(el("clipPath", { id: clip }, el(shape.tag, shape.a)));
    out.push(el(shape.tag, { ...shape.a, fill: "#FFFFFF", ...line }));
    const pr = 0.125 * big * pose.pupil;
    out.push(el("g", { "clip-path": `url(#${clip})` },
      circ(cx + look[0] * 0.065 + (box ? -0.02 : 0), cy + look[1] * 0.065 + (box ? -0.04 : 0), pr, INK, { stroke: "none" }) +
      circ(cx + look[0] * 0.065 + pr * 0.35, cy + look[1] * 0.065 + pr * 0.4, pr * 0.28, "#FFFFFF", { stroke: "none" }) +
      (lid > 0.02
        ? el("g", { transform: `rotate(${-deg(s * pose.tilt)} ${X(cx)} ${Y(cy)})` },
            el("rect", { x: X(cx - 0.4), y: Y(cy + 0.4), width: 0.8 * R, height: (0.4 - ry + ry * lid) * R, fill: skin }) +
            el("line", { x1: X(cx - 0.4), x2: X(cx + 0.4), y1: Y(cy + ry - ry * lid), y2: Y(cy + ry - ry * lid), ...line }))
        : "")));
    out.push(el(shape.tag, { ...shape.a, fill: "none", ...line }));
  });

  // ---------- brows ----------
  const browColor = hasBeard ? shade(d.beardColor, 0.9) : shade(d.skin, 0.55);
  if (!["gnome", "wizard", "beanie"].includes(d.hat)) [-1, 1].forEach((s, i) => {
    const [lift, slant] = i === 0 && pose.browL ? pose.browL : pose.brow;
    const cx = s * EX, cy = EY + (d.eyes === "oval" ? 0.42 : 0.36) + lift;
    out.push(el("line", { x1: X(cx - 0.17), x2: X(cx + 0.17), y1: Y(cy), y2: Y(cy), stroke: INK, "stroke-width": 11, "stroke-linecap": "round", transform: `rotate(${-deg(s * slant)} ${X(cx)} ${Y(cy)})` }));
    out.push(el("line", { x1: X(cx - 0.17), x2: X(cx + 0.17), y1: Y(cy), y2: Y(cy), stroke: browColor, "stroke-width": 5, "stroke-linecap": "round", transform: `rotate(${-deg(s * slant)} ${X(cx)} ${Y(cy)})` }));
  });

  if (hasBeard) for (const s of [-1, 1]) out.push(ell(s * 0.2, -0.3, 0.3, 0.12, bc, { transform: `rotate(${-deg(s * 0.35)} ${X(s * 0.2)} ${Y(-0.3)})` }));

  // ---------- mouth ----------
  const MY = hasBeard ? -0.52 : -0.44;
  const ms = still.mouth ?? 1;
  const mx = X(0), my = Y(MY);
  const mouth = (() => {
    switch (pose.mouth) {
      case "grin": return path(`M${mx - 11} ${my - 2} A11 ${11 * ms} 0 0 0 ${mx + 11} ${my - 2}Z`, INK, { "stroke-width": 2 }) + el("ellipse", { cx: mx, cy: my + 6 * ms, rx: 5, ry: 3, fill: "#E0566A" });
      case "o": return el("ellipse", { cx: mx, cy: my, rx: 5.5, ry: 7 * ms, fill: INK });
      case "flat": return el("line", { x1: mx - 8, x2: mx + 8, y1: my, y2: my, ...line });
      case "frown": return path(`M${mx - 9} ${my + 4} Q${mx} ${my - 6} ${mx + 9} ${my + 4}`, "none");
      case "smirk": return path(`M${mx - 6} ${my} Q${mx + 3} ${my + 5} ${mx + 10} ${my - 3}`, "none");
      default: return path(`M${mx - 9} ${my - 3} Q${mx} ${my + 7} ${mx + 9} ${my - 3}`, "none");
    }
  })();
  out.push(mouth);
  if (d.extras.includes("tusks")) for (const s of [-1, 1]) out.push(path(`M${mx + s * 7} ${my} L${mx + s * 11} ${my - 13} L${mx + s * 14} ${my}Z`, "#F4EFE2", { "stroke-width": 2.5 }));
  if (d.extras.includes("pipe")) out.push(
    el("line", { x1: mx + 6, y1: my, x2: X(0.55), y2: Y(MY - 0.05), stroke: "#25221F", "stroke-width": 4, "stroke-linecap": "round" }),
    el("rect", { x: X(0.5), y: Y(MY + 0.2), width: 15, height: 18, rx: 4, fill: "#6B4226", ...line, "stroke-width": 3 }),
  );

  // ---------- nose (on top of the moustache) ----------
  const NY = -0.08;
  if (d.nose === "bulb") out.push(ell(0, NY, 0.23, 0.21, dark));
  else if (d.nose === "button") out.push(ell(0, NY, 0.14, 0.12, dark));
  else if (d.nose === "long") out.push(path(`M${X(-0.12)} ${Y(NY + 0.1)} Q${X(0)} ${Y(NY + 0.22)} ${X(0.12)} ${Y(NY + 0.1)} L${X(0.04)} ${Y(NY - 0.45)} Q${X(0)} ${Y(NY - 0.52)} ${X(-0.04)} ${Y(NY - 0.45)}Z`, dark));
  else out.push(path(`M${X(-0.15)} ${Y(NY + 0.12)} Q${X(0.05)} ${Y(NY + 0.25)} ${X(0.17)} ${Y(NY)} Q${X(0.12)} ${Y(NY - 0.3)} ${X(-0.12)} ${Y(NY - 0.32)} Q${X(0.02)} ${Y(NY - 0.12)} ${X(-0.15)} ${Y(NY + 0.12)}Z`, dark));

  // ---------- eyewear ----------
  if (d.eyewear === "round" || d.eyewear === "shades") {
    const shades = d.eyewear === "shades";
    for (const s of [-1, 1]) {
      out.push(circ(s * EX, EY, shades ? 0.27 : 0.25, shades ? INK : "none", { "stroke-width": shades ? SW : 3.5 }));
      if (shades) out.push(path(`M${X(s * EX - 0.12)} ${Y(EY + 0.05)} Q${X(s * EX - 0.08)} ${Y(EY + 0.16)} ${X(s * EX + 0.02)} ${Y(EY + 0.18)}`, "none", { stroke: "#FFFFFF", "stroke-width": 3 }));
      out.push(el("line", { x1: X(s * (EX + 0.25)), y1: Y(EY), x2: X(s * hx * 0.98), y2: Y(EY + 0.04), ...line, "stroke-width": 3.5 }));
    }
    out.push(path(`M${X(-EX + 0.25)} ${Y(EY)} Q${X(0)} ${Y(EY + 0.12)} ${X(EX - 0.25)} ${Y(EY)}`, "none", { "stroke-width": 3.5 }));
  } else if (d.eyewear === "monocle") {
    out.push(circ(EX, EY, 0.27, "none", { stroke: "#E0A92B", "stroke-width": 5 }), circ(EX, EY, 0.27, "none", { "stroke-width": 1.5 }));
    out.push(path(`M${X(EX + 0.2)} ${Y(EY - 0.18)} Q${X(EX + 0.45)} ${Y(EY - 0.6)} ${X(EX + 0.3)} ${Y(EY - 1.0)}`, "none", { stroke: "#E0A92B", "stroke-width": 2.5, "stroke-dasharray": "3 4" }));
  }

  // ---------- hats ----------
  const hc = d.hatColor, hcDark = shade(hc, 0.78);
  const capY = 0.48 * hy;
  if (d.hat === "gnome" || d.hat === "wizard") {
    const wiz = d.hat === "wizard";
    const tipX = wiz ? -0.75 : 0.6, tipY = capY + (wiz ? 2.3 : 2.0);
    const w = hx * 1.04;
    if (wiz) out.push(ell(0, capY + 0.02, hx * 1.65, 0.14, hc));
    out.push(path(`M${X(-w)} ${Y(capY)} Q${X(-w * 0.55)} ${Y(capY + 1.3)} ${X(tipX)} ${Y(tipY)} Q${X(w * 0.85)} ${Y(capY + 1.1)} ${X(w)} ${Y(capY)} Q${X(0)} ${Y(capY - 0.12)} ${X(-w)} ${Y(capY)}Z`, hc));
    out.push(path(`M${X(w * 0.25)} ${Y(capY + 0.15)} Q${X(w * 0.75)} ${Y(capY + 1.0)} ${X(tipX)} ${Y(tipY)} Q${X(w * 0.85)} ${Y(capY + 1.1)} ${X(w)} ${Y(capY)}Z`, hcDark, { stroke: "none" }));
    out.push(path(`M${X(-w)} ${Y(capY)} Q${X(-w * 0.55)} ${Y(capY + 1.3)} ${X(tipX)} ${Y(tipY)} Q${X(w * 0.85)} ${Y(capY + 1.1)} ${X(w)} ${Y(capY)}`, "none"));
    if (wiz) {
      out.push(el("rect", { x: X(-w * 0.98), y: Y(capY + 0.3), width: w * 1.96 * R, height: 0.18 * R, fill: "#E0A92B", ...line, "stroke-width": 3 }));
      out.push(star(X(tipX), Y(tipY), 14, "#F2B630"));
    } else out.push(el("rect", { x: X(-w * 1.0), y: Y(capY + 0.12), width: w * 2 * R, height: 0.24 * R, rx: 0.12 * R, fill: hcDark, ...line }));
  } else if (d.hat === "beret") {
    out.push(el("rect", { x: X(-0.06), y: Y(hy * 0.7 + 0.5), width: 6, height: 14, rx: 3, fill: hc, ...line, "stroke-width": 3 }));
    out.push(ell(-0.08, hy * 0.7, hx * 1.14 * 1.08, 0.38, hc, { transform: `rotate(-12 ${X(-0.08)} ${Y(hy * 0.7)})` }));
  } else if (d.hat === "beanie") {
    const rx = 1.06 * hx * 1.02, base = capY - 0.15;
    out.push(path(`M${X(-rx)} ${Y(base)} A${rx * R} ${1.06 * hy * R} 0 0 1 ${X(rx)} ${Y(base)}Z`, hc));
    for (const k of [-0.5, 0, 0.5]) out.push(el("line", { x1: X(k * rx), x2: X(k * rx * 0.7), y1: Y(base + 0.1), y2: Y(base + 0.85), stroke: hcDark, "stroke-width": 3, "stroke-linecap": "round" }));
    out.push(el("rect", { x: X(-hx * 1.1), y: Y(capY + 0.07), width: hx * 2.2 * R, height: 0.3 * R, rx: 8, fill: hcDark, ...line }));
    out.push(circ(0, base + 1.06 * hy + 0.05, 0.26, "#ECE6D6"));
  } else if (d.hat === "mushroom") {
    const rx = 1.5 * hx * 1.02, ry = 1.5 * 0.75, base = capY + 0.05;
    out.push(ell(0, base - 0.02, rx * 0.92, 0.13, "#F2ECDD"));
    out.push(path(`M${X(-rx)} ${Y(base)} A${rx * R} ${ry * R} 0 0 1 ${X(rx)} ${Y(base)} Q${X(0)} ${Y(base - 0.12)} ${X(-rx)} ${Y(base)}Z`, hc));
    for (const [sx2, sy2, r] of [[0, 0.75, 0.2], [0.75, 0.45, 0.16], [-0.8, 0.5, 0.18], [0.35, 1.0, 0.12], [-0.4, 1.0, 0.13], [1.15, 0.15, 0.1], [-1.2, 0.18, 0.11]])
      out.push(circ(sx2 * hx, base + sy2 * 0.95, r, "#F7F2E6", { "stroke-width": 2.5 }));
  } else if (d.hat === "sprout") {
    for (const [x, h, rz] of [[-0.25, 0.4, 0.5], [0.25, 0.4, -0.5], [0, 0.5, 0]] as const)
      out.push(ell(x, hy + h * 0.55, 0.15, h * 0.6, hc, { transform: `rotate(${-deg(rz)} ${X(x)} ${Y(hy + h * 0.55)})` }));
  } else if (d.hat === "headband") {
    out.push(el("rect", { x: X(-hx * 0.93), y: Y(capY + 0.1), width: hx * 1.86 * R, height: 0.2 * R, rx: 5, fill: hc, ...line }));
    out.push(path(`M${X(hx * 0.8)} ${Y(capY)} l14 10 l4 -14Z`, hc, { "stroke-width": 3 }));
  }

  // ---------- headphones, earring ----------
  if (d.extras.includes("headphones")) {
    const tall = d.hat === "gnome" || d.hat === "wizard" || d.hat === "mushroom";
    const r = hx * (tall ? 1.18 : 1.1);
    out.push(path(`M${X(-hx * 1.04)} ${Y(0.1)} C${X(-r)} ${Y(hy + (tall ? 0.6 : 0.35))} ${X(r)} ${Y(hy + (tall ? 0.6 : 0.35))} ${X(hx * 1.04)} ${Y(0.1)}`, "none", { stroke: INK, "stroke-width": 9 }));
    out.push(path(`M${X(-hx * 1.04)} ${Y(0.1)} C${X(-r)} ${Y(hy + (tall ? 0.6 : 0.35))} ${X(r)} ${Y(hy + (tall ? 0.6 : 0.35))} ${X(hx * 1.04)} ${Y(0.1)}`, "none", { stroke: "#3A3832", "stroke-width": 4 }));
    for (const s of [-1, 1]) out.push(el("rect", { x: X(s * hx * 1.04) - 11, y: Y(0.1) - 22, width: 22, height: 44, rx: 10, fill: "#ECE6D6", ...line }), el("rect", { x: X(s * hx * 1.04) - 5 + s * 5, y: Y(0.1) - 15, width: 6, height: 30, rx: 3, fill: "#25221F" }));
  }
  if (d.extras.includes("earring")) back.push(circ(-hx * 1.0, -0.12, 0.1, "none", { stroke: "#E0A92B", "stroke-width": 4 }));

  // ---------- hands ----------
  const hands = opts.hands !== false && !!ex.hands;
  const handPos = still.hands ?? [null, null];
  if (hands) [-1, 1].forEach((s, i) => {
    const p = handPos[i] ?? [s * (hx + 0.2), -1.2];
    front.push(ell(p[0], p[1], 0.21, 0.2, skin));
  });

  // ---------- props ----------
  const topY = d.hat === "gnome" || d.hat === "wizard" ? capY + 1.6 : d.hat === "mushroom" ? capY + 1.1 : hy + 0.3;
  if (opts.props !== false) for (const p of ex.props ?? []) {
    if (p === "zzz") [[0.9, hy + 0.2, 0.7], [1.25, hy + 0.65, 0.9], [1.6, hy + 1.15, 1.1]].forEach(([x, y, s]) => front.push(zee(X(x * Math.max(1, hx)), Y(y), s * 12)));
    if (p === "hearts") [[-0.9, hy + 0.3, 1], [0.95, hy + 0.55, 0.8], [0.1, hy + 1.1, 0.65]].forEach(([x, y, s]) => front.push(heart(X(x), Y(y), s * 16)));
    if (p === "question") front.push(question(X(hx * 0.95), Y(topY + 0.1)));
    if (p === "exclaim") front.push(el("g", { transform: `translate(${X(-hx - 0.35)} ${Y(hy + 0.3)}) rotate(9)` }, el("rect", { x: -5, y: -22, width: 10, height: 26, rx: 4, fill: "#E2443A", ...line, "stroke-width": 3 }) + el("circle", { cx: 0, cy: 14, r: 5.5, fill: "#E2443A", ...line, "stroke-width": 3 })));
    if (p === "dots") [[0.08, 0], [0.12, 1], [0.17, 2]].forEach(([r, k]) => front.push(circ(hx + 0.35 + k * 0.25, hy + 0.35 + k * 0.22, r, "#F7F4EC", { "stroke-width": 3 })));
    if (p === "sweat") front.push(dropPath(X(hx * 0.85), Y(0.4), 1.3));
    if (p === "tear") front.push(dropPath(X(-EX - 0.05), Y(EY - 0.35), 1));
    if (p === "sparkles") [[-1.5, 0.9], [1.55, 1.0], [-1.3, -0.6], [1.45, -0.4], [0, hy + 0.55]].forEach(([x, y], k) => front.push(star(X(x), Y(y), k % 2 ? 9 : 12, "#F2B630", 4)));
    if (p === "magnifier" && hands) {
      const [hx2, hy2] = handPos[1] ?? [hx + 0.2, -1.2];
      front.unshift(
        el("line", { x1: X(hx2), y1: Y(hy2), x2: X(hx2 - 0.15), y2: Y(hy2 + 0.45), stroke: "#6B4226", "stroke-width": 7, "stroke-linecap": "round" }),
        el("circle", { cx: X(hx2 - 0.22), cy: Y(hy2 + 0.68), r: 16, fill: "#CFF1FF", "fill-opacity": 0.55, stroke: "#3A3A44", "stroke-width": 6 }),
      );
    }
  }

  const tilt = still.tilt ?? 0;
  const body = el("g", { transform: tilt ? `rotate(${-deg(tilt)} 120 160)` : undefined }, back.join("") + out.join(""));
  const bg = opts.background ? el("rect", { x: -40, y: -30, width: 320, height: 320, fill: opts.background }) : "";
  const size = opts.size ? { width: opts.size, height: opts.size } : {};
  return el("svg", { xmlns: "http://www.w3.org/2000/svg", viewBox: "-40 -30 320 320", ...size, role: "img", "aria-label": `${d.name}, ${ex.label.toLowerCase()}` },
    bg + body + front.join(""));
}

function star(cx: number, cy: number, r: number, fill: string, sw = 3) {
  const pts: string[] = [];
  for (let i = 0; i < 8; i++) {
    const a = (i * Math.PI) / 4 - Math.PI / 2, rr = i % 2 ? r * 0.38 : r;
    pts.push(`${(cx + Math.cos(a) * rr).toFixed(1)},${(cy + Math.sin(a) * rr).toFixed(1)}`);
  }
  return el("polygon", { points: pts.join(" "), fill, ...line, "stroke-width": sw });
}
function heart(cx: number, cy: number, s: number) {
  return el("path", { d: `M${cx} ${cy + s * 0.9} C${cx - s * 1.4} ${cy} ${cx - s * 0.8} ${cy - s * 0.95} ${cx} ${cy - s * 0.35} C${cx + s * 0.8} ${cy - s * 0.95} ${cx + s * 1.4} ${cy} ${cx} ${cy + s * 0.9}Z`, fill: "#E2443A", ...line, "stroke-width": 3 });
}
function zee(cx: number, cy: number, s: number) {
  return el("path", { d: `M${cx - s} ${cy - s} H${cx + s} L${cx - s} ${cy + s} H${cx + s}`, fill: "none", ...line, "stroke-width": Math.max(3, s * 0.35) });
}
function question(cx: number, cy: number) {
  return el("g", { transform: `translate(${cx} ${cy})` },
    el("path", { d: "M-11 -10 Q-11 -24 0 -24 Q12 -24 12 -12 Q12 -4 3 0 Q0 2 0 8", fill: "none", stroke: INK, "stroke-width": 11, "stroke-linecap": "round", "stroke-linejoin": "round" }) +
    el("path", { d: "M-11 -10 Q-11 -24 0 -24 Q12 -24 12 -12 Q12 -4 3 0 Q0 2 0 8", fill: "none", stroke: "#F2B630", "stroke-width": 5, "stroke-linecap": "round", "stroke-linejoin": "round" }) +
    el("circle", { cx: 0, cy: 20, r: 5.5, fill: "#F2B630", ...line, "stroke-width": 3 }));
}
function dropPath(cx: number, cy: number, s: number) {
  return el("path", { d: `M${cx} ${cy - 12 * s} Q${cx + 8 * s} ${cy} ${cx + 6 * s} ${cy + 4 * s} A6.5 6.5 0 1 1 ${cx - 6 * s} ${cy + 4 * s} Q${cx - 8 * s} ${cy} ${cx} ${cy - 12 * s}Z`, fill: "#7EC8F0", ...line, "stroke-width": 3 });
}
