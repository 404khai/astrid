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
  top: string[]; bottom: string[]; eyes: string; shape: Shape; form?: Form; corners?: Corners; fluffy?: boolean; middle?: string[];
  mood?: Mood; face?: Face; size?: number | string; title?: string; blink?: boolean;
  look?: { x: number; y: number }; background?: string | null;
  /** Realistic fur and glossy eyes (fluffy marks at large sizes). */
  fur?: boolean;
  /** Draw only the body or only the eyes, so the two can sit on separate 3D layers. */
  layer?: "all" | "body" | "eyes";
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

export const FUR_VIEW = "16 56 480 400";

/**
 * Fur: noise pushes the outline into fibres, a blurred copy of the silhouette is lit like a dome for volume,
 * fine streaks add strands. Gloss: each eye lit as a little rounded bead with a hot highlight.
 */
function FurFilters({ uid }: { uid: string }) {
  return (
    <>
      <filter id={`${uid}fur`} x="-20%" y="-20%" width="140%" height="140%" colorInterpolationFilters="sRGB">
        {/* puff the outline: blur, then snap the alpha back so corners round off like a stuffed toy */}
        <feGaussianBlur in="SourceGraphic" stdDeviation="11" result="blur" />
        <feComponentTransfer in="blur" result="puff"><feFuncA type="table" tableValues="0 0 0.6 1 1" /></feComponentTransfer>
        {/* big soft lumps plus fine fibres push the edge out */}
        <feTurbulence type="fractalNoise" baseFrequency="0.014" numOctaves="2" seed="4" result="lumps" />
        <feTurbulence type="fractalNoise" baseFrequency="0.75" numOctaves="2" seed="7" result="fine" />
        <feComposite in="lumps" in2="fine" operator="arithmetic" k2="0.45" k3="0.55" result="noise" />
        <feDisplacementMap in="puff" in2="noise" scale="30" xChannelSelector="R" yChannelSelector="G" result="hair" />
        {/* volume: the silhouette lit like a soft dome */}
        <feGaussianBlur in="hair" stdDeviation="30" result="soft" />
        <feDiffuseLighting in="soft" surfaceScale="16" diffuseConstant="1.22" lightingColor="#ffffff" result="light">
          <feDistantLight azimuth="235" elevation="47" />
        </feDiffuseLighting>
        <feComposite in="hair" in2="light" operator="arithmetic" k1="1.1" result="lit" />
        {/* strands: soft streaks, a few catching light, a few in shadow */}
        <feTurbulence type="fractalNoise" baseFrequency="0.55 0.16" numOctaves="3" seed="2" result="strands" />
        <feGaussianBlur in="strands" stdDeviation="0.6" result="silk" />
        <feColorMatrix in="silk" type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 1.5 -0.78" result="sheen" />
        <feComposite in="sheen" in2="hair" operator="in" result="sheenIn" />
        <feColorMatrix in="silk" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 -1.6 0.64" result="dark" />
        <feComposite in="dark" in2="hair" operator="in" result="darkIn" />
        <feMerge><feMergeNode in="lit" /><feMergeNode in="darkIn" /><feMergeNode in="sheenIn" /></feMerge>
      </filter>
      <filter id={`${uid}pit`} x="-10%" y="-10%" width="120%" height="120%">
        <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="11" result="noise" />
        <feDisplacementMap in="SourceGraphic" in2="noise" scale="10" xChannelSelector="R" yChannelSelector="G" result="edge" />
        <feGaussianBlur in="SourceAlpha" stdDeviation="9" result="blur" />
        <feOffset in="blur" dx="5" dy="9" result="off" />
        <feComposite in="edge" in2="off" operator="out" result="lip" />
        <feColorMatrix in="lip" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 0.55 0" result="lipShade" />
        <feMerge><feMergeNode in="edge" /><feMergeNode in="lipShade" /></feMerge>
      </filter>
      <filter id={`${uid}gloss`} x="-30%" y="-30%" width="160%" height="160%" colorInterpolationFilters="sRGB">
        {/* round the eye corners a touch, then light it as a bead */}
        <feGaussianBlur in="SourceGraphic" stdDeviation="3" result="eb" />
        <feComponentTransfer in="eb" result="eye"><feFuncA type="table" tableValues="0 0 1 1" /></feComponentTransfer>
        <feGaussianBlur in="eye" stdDeviation="7" result="bead" />
        <feSpecularLighting in="bead" surfaceScale="8" specularConstant="1.6" specularExponent="38" lightingColor="#ffffff" result="spec">
          <feDistantLight azimuth="225" elevation="48" />
        </feSpecularLighting>
        <feComposite in="spec" in2="eye" operator="in" result="hi" />
        <feDiffuseLighting in="bead" surfaceScale="5" diffuseConstant="1" lightingColor="#ffffff" result="round">
          <feDistantLight azimuth="225" elevation="60" />
        </feDiffuseLighting>
        <feComposite in="eye" in2="round" operator="arithmetic" k1="1.1" result="body" />
        <feDropShadow in="eye" dx="0" dy="5" stdDeviation="4" floodColor="#000" floodOpacity="0.4" result="sunk" />
        <feMerge><feMergeNode in="sunk" /><feMergeNode in="body" /><feMergeNode in="hi" /></feMerge>
      </filter>
    </>
  );
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
export function Astrid({ top, bottom, eyes, shape, form = "classic", corners = "curved", fluffy = false, middle = ["#121212"], mood = "idle", face, size = 48, title, blink, look, background, animate, fur = false, layer = "all" }: Props) {
  // fur works on the smooth outline; the scalloped one is only for small fluffy previews
  const g = geometry(form, corners, fluffy && !fur);
  const uid = "a" + useId().replace(/[^a-zA-Z0-9]/g, "");
  const moving = !!animate && !face && ANIMATED.has(mood);
  const t = useClock(moving);
  const f = face ?? (moving ? faceAt(mood, t) : FACES[mood]);
  return (
    <svg className={`astrid${blink ? " blinks" : ""}${fur ? " furred" : ""}`} width={size} height={size} viewBox={background ? "0 0 512 512" : fur ? FUR_VIEW : "40 80 432 352"}
      role={title ? "img" : undefined} aria-label={title} aria-hidden={title ? undefined : true} xmlns="http://www.w3.org/2000/svg">
      <defs>
        <Fill id={`${uid}t`} stops={top} x1={131.776} y1={110.848} x2={415.488} y2={229.312} />
        <Fill id={`${uid}b`} stops={bottom} x1={93.376} y1={287.488} x2={369.024} y2={423.808} />
        <Fill id={`${uid}f`} stops={top} x1={93.376} y1={110.848} x2={415.488} y2={423.808} />
        <clipPath id={`${uid}c`}><path d={g.full} /></clipPath>
        <Fill id={`${uid}m`} stops={middle} x1={93.376} y1={110.848} x2={415.488} y2={423.808} />
        {fur && <FurFilters uid={uid} />}
      </defs>
      {background && layer !== "eyes" && <rect width="512" height="512" fill={background} />}
      {layer !== "eyes" && <g filter={fur ? `url(#${uid}fur)` : undefined}>
      {shape === "arches" && <><path d={g.top} fill={`url(#${uid}t)`} /><path d={g.bottom} fill={`url(#${uid}b)`} /></>}
      {shape !== "arches" && <path d={g.full} fill={`url(#${uid}f)`} />}
      {/* split arches in dual: the bottom arch keeps its own stops */}
      {shape === "dual" && bottom.join() !== top.join() && <path d={g.bottom} fill={`url(#${uid}b)`} clipPath={`url(#${uid}c)`} />}
      </g>}
      {/* the dual window is cut into the fur: fuzzy edge, shadowed lip */}
      {shape === "dual" && layer !== "eyes" && <path d={g.window} fill={`url(#${uid}m)`} filter={fur ? `url(#${uid}pit)` : undefined} />}
      {layer !== "body" && <g transform={look ? `translate(${look.x.toFixed(1)} ${look.y.toFixed(1)})` : undefined}>
        <g className="astrid-eyes" fillRule="evenodd" filter={fur ? `url(#${uid}gloss)` : undefined}>
          {(["L", "R"] as const).map((side) => f[side].ghost?.map((gh, i) => <path key={`${side}${i}`} d={eyePath(side, gh)} fill={shade(eyes)} />))}
          <path d={eyePath("L", f.L)} fill={eyes} />
          <path d={eyePath("R", f.R)} fill={eyes} />
        </g>
      </g>}
    </svg>
  );
}
