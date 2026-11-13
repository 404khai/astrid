import { StrictMode, useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource/space-grotesk/latin-400.css";
import "@fontsource/space-grotesk/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import { ThinkingOrb } from "thinking-orbs";
import type { OrbState } from "thinking-orbs";
import { AstridLogo, type EyeExpression } from "../components/AstridLogo";
import { astridCursor } from "../components/cursors";
import { anchor, ShapeView, type Shape } from "./shapes";
import { flatten, ISO_CSS, solid, type Solid } from "./solids";
import { demo } from "./demo";
import "./canvas.css";

/** Astrid's live canvas (ADR 0036). The presence app shows this page in its
 * own panel and relays the runtime's validated canvas events to it. */
type Incoming =
  | { type: "canvas"; event: "begin"; width: number; height: number }
  | { type: "canvas"; event: "shape"; shape: Shape }
  | { type: "canvas"; event: "end" | "cancel" }
  | { type: "answer"; text: string };

const W = 1000;
const H = 640;

/** The design phases the person sees while Astrid works. */
type Phase = "weaving" | "composing" | "sketching" | "done";
const phases: Record<Phase, { orb: OrbState; eyes: EyeExpression; label: string }> = {
  weaving: { orb: "weaving", eyes: "weaving", label: "Weaving" },
  composing: { orb: "composing", eyes: "composing", label: "Composing" },
  sketching: { orb: "shaping", eyes: "sketching", label: "Sketching" },
  done: { orb: "breathing", eyes: "idle", label: "Done" },
};

const native = () =>
  (window as { webkit?: { messageHandlers?: { astrid?: { postMessage(m: unknown): void } } } }).webkit?.messageHandlers
    ?.astrid;
let listener: (m: Incoming) => void = () => {};
(window as { astrid?: unknown }).astrid = { receive: (m: Incoming) => listener(m) };

const reducedMotion = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

const palette: Record<string, string> = {
  ink: "#e6e6e6", paper: "#1d1d1f", blue: "#6575ff", violet: "#9b7bff", pink: "#ec6a8a", red: "#f94447",
  orange: "#ff8a3d", yellow: "#f7c600", green: "#30a46c", teal: "#00c2b8", grey: "#8a8a8a",
};
/** A highlighted solid: the kit's tone variables, mixed from one colour. */
function tint(fill?: string): CSSProperties | undefined {
  const c = fill && fill !== "none" ? palette[fill] ?? fill : null;
  if (!c) return undefined;
  const mix = (n: number) => `color-mix(in srgb, ${c} ${n}%, #000)`;
  return {
    "--anatomy-paper": mix(55),
    "--anatomy-top": mix(100),
    "--anatomy-shade-0": mix(45),
    "--anatomy-shade-1": mix(58),
    "--anatomy-shade-2": mix(70),
    "--anatomy-shade-3": mix(82),
    "--anatomy-hi": `color-mix(in srgb, ${c} 60%, #fff)`,
  } as CSSProperties;
}

function App() {
  const [title, setTitle] = useState<string | null>(null);
  const [drawn, setDrawn] = useState<Shape[]>([]);
  // 3D solids are kept back to front, so a part streamed late still sits
  // behind the parts in front of it.
  const [solids, setSolids] = useState<(Solid & { shape: Shape; n: number })[]>([]);
  const [phase, setPhase] = useState<Phase>("weaving");
  const [cursor, setCursor] = useState<[number, number]>([W / 2, H / 2]);
  const [say, setSay] = useState<string | null>(null);
  const [caption, setCaption] = useState<string | null>(null);
  const queue = useRef<Shape[]>([]);
  const ended = useRef(false);
  const timer = useRef<number | undefined>(undefined);
  const stage = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(1);

  // One shape at a time, so the drawing reads as being made: the cursor
  // travels to each shape, then it is drawn there.
  const pump = () => {
    if (timer.current) return;
    const next = queue.current.shift();
    if (!next) {
      setPhase(ended.current ? "done" : "weaving");
      return;
    }
    setPhase("sketching");
    if (next.shape === "title") {
      setTitle(next.text ?? null);
      timer.current = window.setTimeout(step, 120);
      return;
    }
    const shape = flatten(next);
    const built = solid(shape);
    const at = built?.anchor ?? anchor(shape);
    if (at) setCursor(at);
    if (shape.shape === "point") {
      setSay(shape.say ?? null);
    } else if (built) {
      setSay(null);
      setSolids((list) => [...list, { ...built, shape, n: list.length }].sort((a, b) => a.depth - b.depth || a.n - b.n));
    } else {
      setSay(null);
      setDrawn((list) => [...list, shape]);
    }
    const quick = reducedMotion() || queue.current.length > 40;
    timer.current = window.setTimeout(step, next.shape === "point" ? 1700 : quick ? 140 : 480);
  };
  const step = () => {
    timer.current = undefined;
    pump();
  };

  useEffect(() => {
    listener = (m) => {
      if (m.type === "answer") return setCaption(m.text || null);
      switch (m.event) {
        case "begin":
          window.clearTimeout(timer.current);
          timer.current = undefined;
          queue.current = [];
          ended.current = false;
          setTitle(null);
          setDrawn([]);
          setSolids([]);
          setSay(null);
          setCaption(null);
          setPhase("composing");
          setCursor([W / 2, H / 2]);
          break;
        case "shape":
          queue.current.push(m.shape);
          pump();
          break;
        case "end":
          ended.current = true;
          pump();
          break;
        case "cancel":
          queue.current = [];
          ended.current = true;
          setPhase("done");
          break;
      }
    };
    native()?.postMessage({ type: "ready" });
    if (!native()) demo((m) => listener(m as Incoming));
  }, []);

  useLayoutEffect(() => {
    const el = stage.current;
    if (!el) return;
    const observer = new ResizeObserver(() => setScale(el.clientWidth / W));
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const look = phases[phase];
  const [hx, hy] = astridCursor.hot;
  return (
    <main className="cv-card">
      <header className="cv-head">
        <span className="cv-state" aria-live="polite">
          <AstridLogo expression={look.eyes} />
          <ThinkingOrb state={look.orb} size={20} theme="dark" paused={reducedMotion()} />
          <span>{look.label}</span>
        </span>
        <h1>{title ?? "Canvas"}</h1>
        <button type="button" className="cv-close" aria-label="Close canvas" onClick={() => native()?.postMessage({ type: "canvas-close" })}>
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path d="m4 4 8 8m0-8-8 8" />
          </svg>
        </button>
      </header>
      <style>{ISO_CSS}</style>
      <div className="cv-stage" ref={stage}>
        <svg viewBox={`0 0 ${W} ${H}`} className="cv-svg" role="img" aria-label={title ?? "Astrid's drawing"}>
          <defs>
            <pattern id="cv-dots" width="24" height="24" patternUnits="userSpaceOnUse">
              <circle cx="1" cy="1" r="1" />
            </pattern>
          </defs>
          <rect className="cv-grid" width={W} height={H} fill="url(#cv-dots)" />
          <g className="iso cv-solids" data-theme="dark">
            {solids.map((s) => (
              <g key={s.n} className="cv-solid3" style={tint(s.shape.fill)}>
                <g dangerouslySetInnerHTML={{ __html: s.svg }} />
                {s.shape.label && (
                  <text className="cv-label" x={s.labelAt[0]} y={s.labelAt[1]} textAnchor="middle">
                    {s.shape.label}
                  </text>
                )}
              </g>
            ))}
          </g>
          {drawn.map((shape, index) => (
            <ShapeView key={index} shape={shape} index={index} />
          ))}
        </svg>
        <div className="cv-cursor" style={{ "--x": `${cursor[0] * scale}px`, "--y": `${cursor[1] * scale}px` } as CSSProperties} aria-hidden="true">
          <svg className="cv-tip" viewBox="0 0 32 32" width="30" height="30" style={{ left: -hx * (30 / 32), top: -hy * (30 / 32) }}>
            {astridCursor.art}
          </svg>
          <span className="cv-follow">
            <AstridLogo expression={look.eyes} />
          </span>
          {say && <span className="cv-say">{say}</span>}
        </div>
      </div>
      {caption && <p className="cv-caption">{caption}</p>}
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
