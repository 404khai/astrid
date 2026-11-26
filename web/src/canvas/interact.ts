import type { Shape } from "./shapes";

/** Astrid Weaver interactions: pure helpers over the validated shapes, so the
 * page's behaviour stays easy to test and reason about. */

/** Lines that change or guide a drawing rather than adding a visible part. */
const unseen = new Set<Shape["shape"]>(["title", "canvas", "point", "edit", "step"]);
export const visible = (s: Shape) => !unseen.has(s.shape) && !s.hidden;

/** An edit line merges its fields into every part with its id. */
export function applyEdit(list: Shape[], edit: Shape): Shape[] {
  const { shape: _kind, id, focus: _focus, ...fields } = edit;
  const set = Object.fromEntries(Object.entries(fields).filter(([, v]) => v !== undefined));
  // A value keeps its first reading as the base its driven parts scale from,
  // so "double the flow" doubles their speed.
  return list.map((s) => (s.id === id ? { ...s, ...set, base: s.base ?? s.value } : s));
}

/** A whole saved drawing, with its edits applied in order (replay). */
export function fold(shapes: Shape[]): Shape[] {
  let out: Shape[] = [];
  for (const s of shapes) out = s.shape === "edit" ? applyEdit(out, s) : [...out, s];
  return out;
}

const isWorld = (s: Shape) => s.shape === "slab" || s.shape === "cylinder" || s.shape === "rotor" || s.z !== undefined;

/** Moves a part `t` of the way along its explode offset (0 together, 1 apart). */
export function placed(s: Shape, t: number): Shape {
  if (!s.explode || !t) return s;
  const [dx, dy, dz] = s.explode.map((v) => v * t);
  const shift = (p: number[]) => (p.length === 3 ? [p[0] + dx, p[1] + dy, p[2] + dz] : p);
  const out: Shape = { ...s };
  if (s.points?.length && (s.points[0] as number[]).length === 3) out.points = s.points.map((p) => shift(p) as [number, number]);
  if (s.from && s.from.length === 3) out.from = shift(s.from) as [number, number, number];
  if (s.to && s.to.length === 3) out.to = shift(s.to) as [number, number, number];
  if (isWorld(s) && s.x !== undefined) Object.assign(out, { x: s.x + dx, y: s.y! + dy, z: (s.z ?? 0) + dz });
  return out;
}

/** The live values on the canvas, with any slider override applied. */
export type Live = { id: string; base: number; now: number; shape: Shape };
export function liveValues(drawn: Shape[], overrides: Record<string, number>): Live[] {
  return drawn
    .filter((s) => s.shape === "value" && s.id && !s.hidden)
    .map((s) => ({ id: s.id!, base: s.base ?? s.value ?? 0, now: overrides[s.id!] ?? s.value ?? 0, shape: s }));
}

/** How much the live values speed a part up: the product of now ÷ base for
 * every value that drives it. */
export function driven(s: Shape, values: Live[]): number {
  let factor = 1;
  for (const v of values) {
    if (s.id && v.shape.drives?.includes(s.id) && v.base) factor *= Math.max(0, v.now / v.base);
  }
  return factor;
}

/** The slider range for a value: its own min/max, else 0 to twice it. */
export function range(v: Live): [number, number] {
  const lo = v.shape.min ?? Math.min(0, v.base * 2);
  const hi = v.shape.max ?? Math.max(v.base * 2, lo + 1);
  return [lo, hi];
}
