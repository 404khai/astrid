import type { CSSProperties, ReactNode } from "react";

/** A shape exactly as the runtime validated it (ADR 0036). */
export type Shape = {
  shape: "title" | "rect" | "ellipse" | "line" | "polygon" | "path" | "arrow" | "text" | "box" | "point" | "slab" | "cylinder";
  z?: number;
  radius?: number;
  r2?: number;
  axis?: "x" | "y" | "z";
  id?: string;
  x?: number;
  y?: number;
  w?: number;
  h?: number;
  d?: number | string;
  r?: number;
  cx?: number;
  cy?: number;
  rx?: number;
  ry?: number;
  points?: [number, number][];
  from?: [number, number] | [number, number, number];
  to?: [number, number] | [number, number, number];
  bend?: number;
  text?: string;
  size?: number;
  align?: "start" | "middle" | "end";
  say?: string;
  fill?: string;
  stroke?: string;
  width?: number;
  dashed?: boolean;
  sketchy?: boolean;
  label?: string;
};

const palette: Record<string, string> = {
  ink: "#f4f4f4",
  paper: "#1d1d1f",
  blue: "#6575ff",
  violet: "#9b7bff",
  pink: "#ec6a8a",
  red: "#f94447",
  orange: "#ff8a3d",
  yellow: "#f7c600",
  green: "#30a46c",
  teal: "#00c2b8",
  grey: "#8a8a8a",
  none: "none",
};
const paint = (value: string | undefined, fallback: string) => (value ? palette[value] ?? value : fallback);

/** Deterministic jitter, so a sketchy shape looks drawn but never flickers. */
const noise = (seed: number) => {
  const v = Math.sin(seed * 127.1) * 43758.5453;
  return v - Math.floor(v);
};
const f = (n: number) => n.toFixed(1);

/** A hand-drawn polyline: each segment bows a little and overshoots its end. */
function roughPath(points: [number, number][], seed: number, closed = false) {
  const list = closed ? [...points, points[0]] : points;
  let d = `M${f(list[0][0])} ${f(list[0][1])}`;
  for (let i = 1; i < list.length; i++) {
    const [x1, y1] = list[i - 1];
    const [x2, y2] = list[i];
    const len = Math.hypot(x2 - x1, y2 - y1) || 1;
    const bow = (noise(seed + i) - 0.5) * Math.min(6, len * 0.04);
    const mx = (x1 + x2) / 2 - ((y2 - y1) / len) * bow;
    const my = (y1 + y2) / 2 + ((x2 - x1) / len) * bow;
    const over = 1 + noise(seed + i * 3) * 0.02;
    d += `Q${f(mx)} ${f(my)} ${f(x1 + (x2 - x1) * over)} ${f(y1 + (y2 - y1) * over)}`;
  }
  return d;
}

function ellipsePath(cx: number, cy: number, rx: number, ry: number, seed: number, sketchy: boolean) {
  if (!sketchy) return `M${cx - rx} ${cy}a${rx} ${ry} 0 1 0 ${2 * rx} 0a${rx} ${ry} 0 1 0 ${-2 * rx} 0Z`;
  const steps = 40;
  const turns = 1.08;
  let d = "";
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const a = -2.2 + t * turns * Math.PI * 2;
    const wobble = 1 + (noise(seed + i) - 0.5) * 0.06 + t * 0.03;
    d += `${i ? "L" : "M"}${f(cx + Math.cos(a) * rx * wobble)} ${f(cy + Math.sin(a) * ry * wobble)}`;
  }
  return d;
}

function rectPath(x: number, y: number, w: number, h: number, r = 0) {
  r = Math.min(r, w / 2, h / 2);
  if (!r) return `M${x} ${y}h${w}v${h}h${-w}Z`;
  return `M${x + r} ${y}h${w - 2 * r}a${r} ${r} 0 0 1 ${r} ${r}v${h - 2 * r}a${r} ${r} 0 0 1 ${-r} ${r}h${-(w - 2 * r)}a${r} ${r} 0 0 1 ${-r} ${-r}v${-(h - 2 * r)}a${r} ${r} 0 0 1 ${r} ${-r}Z`;
}

/** Isometric axes for Anatomy-style blocks. */
const C = Math.cos(Math.PI / 6);
const S = 0.5;
function isoBox(x: number, y: number, w: number, d: number, h: number) {
  const pt = (p: [number, number]) => `${f(p[0])} ${f(p[1])}`;
  const front: [number, number] = [x, y];
  const right: [number, number] = [x + w * C, y - w * S];
  const left: [number, number] = [x - d * C, y - d * S];
  const back: [number, number] = [right[0] - d * C, right[1] - d * S];
  const up = (p: [number, number]): [number, number] => [p[0], p[1] - h];
  const face = (ps: [number, number][]) => `M${ps.map(pt).join("L")}Z`;
  return {
    left: face([front, left, up(left), up(front)]),
    right: face([front, right, up(right), up(front)]),
    top: face([up(front), up(right), up(back), up(left)]),
    labelAt: [x + (w * C - d * C) / 2, y - h - (w + d) * S * 0.5] as [number, number],
  };
}

/** Where Astrid's cursor goes while this shape is drawn. */
export function anchor(s: Shape): [number, number] | null {
  switch (s.shape) {
    case "rect":
      return [s.x! + s.w!, s.y! + s.h! / 2];
    case "ellipse":
      return [s.cx! + s.rx! * 0.7, s.cy! + s.ry! * 0.7];
    case "line":
    case "polygon":
      return s.points![s.points!.length - 1];
    case "arrow":
      return [s.to![0], s.to![1]];
    case "text":
    case "point":
      return [s.x!, s.y!];
    case "box":
      return [s.x!, s.y! - s.h! / 2];
    case "path": {
      const nums = (s.d as string).match(/-?\d*\.?\d+(e[-+]?\d+)?/gi)?.map(Number) ?? [];
      return nums.length >= 2 ? [nums[0], nums[1]] : null;
    }
    default:
      return null;
  }
}

function labelPosition(s: Shape): [number, number] | null {
  switch (s.shape) {
    case "rect":
      return [s.x! + s.w! / 2, s.y! + s.h! + 22];
    case "ellipse":
      return [s.cx!, s.cy! + s.ry! + 22];
    case "box":
      return isoBox(s.x!, s.y!, s.w!, s.d as number, s.h!).labelAt.map((v, i) => (i ? v - 12 : v)) as [number, number];
    case "arrow": {
      const [x1, y1] = s.from!;
      const [x2, y2] = s.to!;
      return [(x1 + x2) / 2, (y1 + y2) / 2 - 10];
    }
    default: {
      const a = anchor(s);
      return a && [a[0], a[1] - 14];
    }
  }
}

/** One validated shape, drawn by a trusted template. Text is always text. */
export function ShapeView({ shape: s, index }: { shape: Shape; index: number }) {
  const seed = index * 7 + 3;
  const stroke = paint(s.stroke, "#f4f4f4");
  const style = {
    "--i": index,
    strokeWidth: s.width ?? 2,
    strokeDasharray: s.dashed ? "7 7" : undefined,
  } as CSSProperties;
  const cls = `cv-shape cv-${s.shape}${s.dashed ? " cv-dashed" : ""}`;
  let body: ReactNode = null;
  switch (s.shape) {
    case "rect": {
      const d = s.sketchy
        ? roughPath([[s.x!, s.y!], [s.x! + s.w!, s.y!], [s.x! + s.w!, s.y! + s.h!], [s.x!, s.y! + s.h!]], seed, true)
        : rectPath(s.x!, s.y!, s.w!, s.h!, s.r);
      body = <path d={d} pathLength={1} fill={paint(s.fill, "none")} stroke={stroke} />;
      break;
    }
    case "ellipse":
      body = <path d={ellipsePath(s.cx!, s.cy!, s.rx!, s.ry!, seed, !!s.sketchy)} pathLength={1} fill={paint(s.fill, "none")} stroke={stroke} />;
      break;
    case "line":
    case "polygon": {
      const closed = s.shape === "polygon";
      const d = s.sketchy ? roughPath(s.points!, seed, closed) : `M${s.points!.map((p) => p.join(" ")).join("L")}${closed ? "Z" : ""}`;
      body = <path d={d} pathLength={1} fill={closed ? paint(s.fill, "none") : "none"} stroke={stroke} />;
      break;
    }
    case "path":
      body = <path d={s.d as string} pathLength={1} fill={paint(s.fill, "none")} stroke={stroke} />;
      break;
    case "arrow": {
      const [x1, y1] = s.from!;
      const [x2, y2] = s.to!;
      const bend = s.bend ?? (s.sketchy ? 0.18 : 0);
      const mx = (x1 + x2) / 2 - (y2 - y1) * bend;
      const my = (y1 + y2) / 2 + (x2 - x1) * bend;
      const angle = Math.atan2(y2 - my, x2 - mx);
      const head = (o: number) => `M${x2} ${y2}L${f(x2 - Math.cos(angle + o) * 13)} ${f(y2 - Math.sin(angle + o) * 13)}`;
      body = (
        <>
          <path d={`M${x1} ${y1}Q${f(mx)} ${f(my)} ${x2} ${y2}`} pathLength={1} fill="none" stroke={stroke} />
          <path className="cv-arrowhead" d={head(0.45) + head(-0.45)} fill="none" stroke={stroke} />
        </>
      );
      break;
    }
    case "box": {
      const fill = paint(s.fill, "#6575ff");
      const faces = isoBox(s.x!, s.y!, s.w!, s.d as number, s.h!);
      const shade = (amount: number) => (fill === "none" ? "none" : `color-mix(in srgb, ${fill} ${amount}%, #000)`);
      body = (
        <>
          <path d={faces.left} pathLength={1} style={{ fill: shade(62) }} stroke={paint(s.stroke, "#0008")} />
          <path d={faces.right} pathLength={1} style={{ fill: shade(80) }} stroke={paint(s.stroke, "#0008")} />
          <path d={faces.top} pathLength={1} style={{ fill: shade(100) }} stroke={paint(s.stroke, "#0008")} />
        </>
      );
      break;
    }
    case "text":
      body = (
        <text x={s.x} y={s.y} fontSize={s.size ?? 18} textAnchor={s.align ?? "start"} fill={paint(s.fill ?? s.stroke, "#f4f4f4")} stroke="none">
          {s.text}
        </text>
      );
      break;
    default:
      return null;
  }
  const at = s.label ? labelPosition(s) : null;
  return (
    <g className={cls} style={style}>
      {body}
      {at && (
        <text className="cv-label" x={at[0]} y={at[1]} textAnchor="middle" stroke="none">
          {s.label}
        </text>
      )}
    </g>
  );
}
