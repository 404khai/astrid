import { useEffect, useId, useState } from "react";
import { geometry, type Corners, type Form } from "./forms";
import { ANIMATED, FACES, eyePath, faceAt, type Face, type Mood } from "./faces";

/** Seconds since mount, ticking every frame while `on`; 0 under reduced motion. */
function useClock(on: boolean) {
  const [t, setT] = useState(0);
  useEffect(() => {
    if (!on || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const start = performance.now();
    let id = requestAnimationFrame(function tick(now) { setT(Math.max(0, now - start) / 1000); id = requestAnimationFrame(tick); });
    return () => cancelAnimationFrame(id);
  }, [on]);
  return t;
}

export type Shape = "arches" | "full" | "dual";
interface Props {
  top: string[]; bottom: string[]; eyes: string; shape: Shape; form?: Form; corners?: Corners; middle?: string[];
  mood?: Mood; face?: Face; size?: number | string; title?: string; blink?: boolean;
  look?: { x: number; y: number }; background?: string | null;
  /** Play the mood's motion (orbiting, calculating, searching, scanning, thinking). */
  animate?: boolean;
}

/** A darker shade of the eye colour for unlit parts; very dark eyes get a lighter one so it still shows. */
function shade(hex: string) {
  const n = parseInt(hex.slice(1), 16), c = [n >> 16, (n >> 8) & 255, n & 255];
  const lum = (0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]) / 255;
  const out = c.map((v) => Math.round(lum < 0.2 ? v + (255 - v) * 0.3 : v * 0.42));
  return "#" + out.map((v) => v.toString(16).padStart(2, "0")).join("");
}

// Same gradient axes as references/astrid.svg; stops are spread evenly along them.
function Fill({ id, stops, x1, y1, x2, y2 }: { id: string; stops: string[]; x1: number; y1: number; x2: number; y2: number }) {
  return (
    <linearGradient id={id} x1={x1} y1={y1} x2={x2} y2={y2} gradientUnits="userSpaceOnUse">
      {stops.map((c, i) => <stop key={i} offset={stops.length === 1 ? 0 : i / (stops.length - 1)} stopColor={c} />)}
    </linearGradient>
  );
}

/** The Astrid mark: two arches, the joined full shape, or dual (the full shape with its eye window in its own colour) and eyes re-cut for a mood. */
export function Astrid({ top, bottom, eyes, shape, form = "classic", corners = "curved", middle = ["#121212"], mood = "idle", face, size = 48, title, blink, look, background, animate }: Props) {
  const g = geometry(form, corners);
  const uid = "a" + useId().replace(/[^a-zA-Z0-9]/g, "");
  const moving = !!animate && !face && ANIMATED.has(mood);
  const t = useClock(moving);
  const f = face ?? (moving ? faceAt(mood, t) : FACES[mood]);
  return (
    <svg className={`astrid${blink ? " blinks" : ""}`} width={size} height={size} viewBox={background ? "0 0 512 512" : "40 80 432 352"}
      role={title ? "img" : undefined} aria-label={title} aria-hidden={title ? undefined : true} xmlns="http://www.w3.org/2000/svg">
      <defs>
        <Fill id={`${uid}t`} stops={top} x1={131.776} y1={110.848} x2={415.488} y2={229.312} />
        <Fill id={`${uid}b`} stops={bottom} x1={93.376} y1={287.488} x2={369.024} y2={423.808} />
        <Fill id={`${uid}f`} stops={top} x1={93.376} y1={110.848} x2={415.488} y2={423.808} />
        <clipPath id={`${uid}c`}><path d={g.full} /></clipPath>
        <Fill id={`${uid}m`} stops={middle} x1={93.376} y1={110.848} x2={415.488} y2={423.808} />
      </defs>
      {background && <rect width="512" height="512" fill={background} />}
      {shape === "arches" && <><path d={g.top} fill={`url(#${uid}t)`} /><path d={g.bottom} fill={`url(#${uid}b)`} /></>}
      {shape !== "arches" && <path d={g.full} fill={`url(#${uid}f)`} />}
      {/* split arches in dual: the bottom arch keeps its own stops */}
      {shape === "dual" && bottom.join() !== top.join() && <path d={g.bottom} fill={`url(#${uid}b)`} clipPath={`url(#${uid}c)`} />}
      {shape === "dual" && <path d={g.window} fill={`url(#${uid}m)`} />}
      <g transform={look ? `translate(${look.x.toFixed(1)} ${look.y.toFixed(1)})` : undefined}>
        <g className="astrid-eyes" fillRule="evenodd">
          {(["L", "R"] as const).map((side) => f[side].ghost?.map((gh, i) => <path key={`${side}${i}`} d={eyePath(side, gh)} fill={shade(eyes)} />))}
          <path d={eyePath("L", f.L)} fill={eyes} />
          <path d={eyePath("R", f.R)} fill={eyes} />
        </g>
      </g>
    </svg>
  );
}
