import { StrictMode, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource/space-grotesk/latin-400.css";
import "@fontsource/space-grotesk/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import { ThinkingOrb } from "thinking-orbs";
import type { OrbState } from "thinking-orbs";
import { AstridLogo, type EyeExpression } from "../components/AstridLogo";
import { astridCursor } from "../components/cursors";
import { anchor, ShapeView, type Shape } from "./shapes";
import { defaultCamera, flatten, ISO_CSS, project, solid, type Camera } from "./solids";
import { demo } from "./demo";
import "./canvas.css";

/** Astrid's live canvas (ADR 0036). The presence app shows this page in its
 * own panel and relays the runtime's validated canvas events to it, plus the
 * ask surface's state so a follow-up shows on the canvas too. */
type Incoming =
  | { type: "canvas"; event: "begin"; width: number; height: number }
  | { type: "canvas"; event: "shape"; shape: Shape }
  | { type: "canvas"; event: "end" | "cancel" }
  | { type: "state"; state: string }
  | { type: "listening"; on: boolean }
  | { type: "done" };

const W = 1000;
const H = 640;

/** What Astrid is doing, as the docs' agent states: orb beside the mark. */
type Activity = "listening" | "working" | "searching" | "weaving" | "composing" | "sketching";
const looks: Record<Activity, { orb: OrbState; eyes: EyeExpression }> = {
  listening: { orb: "listening", eyes: "listening" },
  working: { orb: "working", eyes: "working" },
  searching: { orb: "searching", eyes: "searching" },
  weaving: { orb: "weaving", eyes: "weaving" },
  composing: { orb: "composing", eyes: "composing" },
  sketching: { orb: "shaping", eyes: "sketching" },
};
const activityOf = (state: string): Activity | null => (state in looks ? (state as Activity) : null);

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

/** Where the cursor points: a spot on the canvas, or a spot in the 3D world
 * that moves with the figure when you turn it. */
type Target = { flat: [number, number] } | { world: [number, number, number] };
type View = { x: number; y: number; zoom: number };
const home: View = { x: 0, y: 0, zoom: 1 };

function App() {
  const [title, setTitle] = useState<string | null>(null);
  const [drawn, setDrawn] = useState<Shape[]>([]);
  const [drawing, setDrawing] = useState<Activity | null>(null);
  const [external, setExternal] = useState<Activity | null>(null);
  const [target, setTarget] = useState<Target>({ flat: [W / 2, H / 2] });
  const [say, setSay] = useState<string | null>(null);
  const [cam, setCam] = useState<Camera>(defaultCamera);
  const [view, setView] = useState<View>(home);
  const [moving, setMoving] = useState(false);
  const [size, setSize] = useState({ w: W, h: H });
  const queue = useRef<Shape[]>([]);
  const ended = useRef(true);
  const touched = useRef(false);
  const timer = useRef<number | undefined>(undefined);
  const stage = useRef<HTMLDivElement>(null);
  const content = useRef<SVGGElement>(null);

  // Built from the raw shapes for the current camera, so turning the figure
  // re-projects every solid, label and arrow. Solids are drawn back to front.
  const scene = useMemo(() => {
    const solids = drawn
      .map((shape, n) => ({ shape, n, built: solid(shape, cam) }))
      .filter((s) => s.built)
      .map((s) => ({ ...s, built: s.built! }))
      .sort((a, b) => a.built.depth - b.built.depth || a.n - b.n);
    const flats = drawn.map((shape, n) => ({ shape: flatten(shape, cam), n })).filter((s) => s.shape.shape !== "slab" && s.shape.shape !== "cylinder");
    return { solids, flats, last: drawn.length - 1 };
  }, [drawn, cam]);

  /** Frames everything drawn so far; while drawing it only ever zooms out. */
  const fit = (outOnly: boolean) => {
    const box = content.current?.getBBox();
    if (!box || !box.width || !box.height) return;
    const zoom = Math.min((W * 0.84) / box.width, (H * 0.74) / box.height, 1.4);
    setView((v) => {
      if (outOnly && zoom >= v.zoom) {
        const left = v.x + box.x * v.zoom, right = left + box.width * v.zoom;
        const top = v.y + box.y * v.zoom, bottom = top + box.height * v.zoom;
        if (left >= 20 && right <= W - 20 && top >= 70 && bottom <= H - 20) return v;
      }
      const z = outOnly ? Math.min(zoom, v.zoom) : zoom;
      return { zoom: z, x: W / 2 - (box.x + box.width / 2) * z, y: H / 2 + 24 - (box.y + box.height / 2) * z };
    });
  };

  // One shape at a time, so the drawing reads as being made: the cursor
  // travels to each shape, then it is drawn there.
  const pump = () => {
    if (timer.current) return;
    const next = queue.current.shift();
    if (!next) {
      setDrawing(ended.current ? null : "weaving");
      if (ended.current && !touched.current) requestAnimationFrame(() => fit(false));
      return;
    }
    setDrawing("sketching");
    if (next.shape === "title") {
      setTitle(next.text ?? null);
      timer.current = window.setTimeout(step, 120);
      return;
    }
    const built = solid(next);
    if (built) setTarget({ world: built.world });
    else if (next.shape === "point" && next.z !== undefined) setTarget({ world: [next.x!, next.y!, next.z] });
    else if (next.shape === "text" && next.z !== undefined) setTarget({ world: [next.x!, next.y!, next.z] });
    else if (next.shape === "arrow" && next.to!.length === 3) setTarget({ world: next.to as [number, number, number] });
    else {
      const at = anchor(next);
      if (at) setTarget({ flat: at });
    }
    if (next.shape === "point") setSay(next.say ?? null);
    else {
      setSay(null);
      setDrawn((list) => [...list, next]);
      if (!touched.current) requestAnimationFrame(() => fit(true));
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
      switch (m.type) {
        case "state":
          setExternal(m.state === "idle" ? null : activityOf(m.state) ?? "working");
          return;
        case "listening":
          setExternal((a) => (m.on ? "listening" : a === "listening" ? null : a));
          return;
        case "done":
          setExternal(null);
          return;
      }
      switch (m.event) {
        case "begin":
          window.clearTimeout(timer.current);
          timer.current = undefined;
          queue.current = [];
          ended.current = false;
          touched.current = false;
          setTitle(null);
          setDrawn([]);
          setSay(null);
          setDrawing("composing");
          setCam(defaultCamera);
          setView(home);
          setTarget({ flat: [W / 2, H / 2] });
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
          setDrawing(null);
          break;
      }
    };
    native()?.postMessage({ type: "ready" });
    if (!native()) demo((m) => listener(m as Incoming));
  }, []);

  useLayoutEffect(() => {
    const el = stage.current;
    if (!el) return;
    const observer = new ResizeObserver(() => setSize({ w: el.clientWidth, h: el.clientHeight }));
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // The SVG fills the stage, letterboxed: canvas units → stage pixels.
  const unit = Math.min(size.w / W, size.h / H);
  const offset = { x: (size.w - W * unit) / 2, y: (size.h - H * unit) / 2 };
  const toStage = ([x, y]: [number, number]): [number, number] => [
    offset.x + (view.x + x * view.zoom) * unit,
    offset.y + (view.y + y * view.zoom) * unit,
  ];
  const cursor = toStage("world" in target ? project(target.world, cam) : target.flat);
  const hasSolids = scene.solids.length > 0;

  // Drag turns a 3D figure (or pans a flat one); two-finger scroll pans,
  // pinch zooms, double-click puts it back.
  const onPointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0 || (e.target as Element).closest("button, .cv-title")) return;
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    touched.current = true;
    setMoving(true);
    let last = { x: e.clientX, y: e.clientY };
    const turn = hasSolids && !e.shiftKey;
    const move = (ev: PointerEvent) => {
      const dx = ev.clientX - last.x, dy = ev.clientY - last.y;
      last = { x: ev.clientX, y: ev.clientY };
      if (turn) {
        setCam((c) => ({ azimuth: (c.azimuth + dx * 0.4 + 360) % 360, elevation: Math.min(75, Math.max(8, c.elevation + dy * 0.25)) }));
      } else {
        setView((v) => ({ ...v, x: v.x + dx / unit, y: v.y + dy / unit }));
      }
    };
    const up = () => {
      setMoving(false);
      removeEventListener("pointermove", move);
      removeEventListener("pointerup", up);
    };
    addEventListener("pointermove", move);
    addEventListener("pointerup", up);
  };
  const onWheel = (e: React.WheelEvent) => {
    touched.current = true;
    if (e.ctrlKey) {
      // Pinch: zoom around the pointer.
      const rect = stage.current!.getBoundingClientRect();
      const px = (e.clientX - rect.left - offset.x) / unit, py = (e.clientY - rect.top - offset.y) / unit;
      setView((v) => {
        const zoom = Math.min(6, Math.max(0.2, v.zoom * Math.exp(-e.deltaY * 0.01)));
        return { zoom, x: px - ((px - v.x) / v.zoom) * zoom, y: py - ((py - v.y) / v.zoom) * zoom };
      });
    } else {
      setView((v) => ({ ...v, x: v.x - e.deltaX / unit, y: v.y - e.deltaY / unit }));
    }
  };
  const reset = () => {
    touched.current = false;
    setCam(defaultCamera);
    requestAnimationFrame(() => fit(false));
  };

  const activity = drawing ?? external;
  const look = activity ? looks[activity] : null;
  const [hx, hy] = astridCursor.hot;
  const flip = cursor[0] > size.w - 340;
  return (
    <main className={`cv-card${moving ? " cv-moving" : ""}`}>
      <style>{ISO_CSS}</style>
      <div className="cv-stage" ref={stage} onPointerDown={onPointerDown} onWheel={onWheel} onDoubleClick={reset}>
        <svg viewBox={`0 0 ${W} ${H}`} className="cv-svg" role="img" aria-label={title ?? "Astrid's drawing"}>
          <g ref={content} className="cv-view" style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.zoom})` }}>
            <g className="iso cv-solids" data-theme="dark">
              {scene.solids.map(({ shape, n, built }) => (
                <g key={n} className={`cv-solid3${n === scene.last ? " cv-new" : ""}`} style={tint(shape.fill)}>
                  <g dangerouslySetInnerHTML={{ __html: built.svg }} />
                </g>
              ))}
              {scene.solids.map(({ shape, n, built }) =>
                shape.label ? (
                  <text key={`l${n}`} className="cv-label" x={built.labelAt[0]} y={built.labelAt[1]} textAnchor="middle">
                    {shape.label}
                  </text>
                ) : null,
              )}
            </g>
            {scene.flats.map(({ shape, n }) => (
              <ShapeView key={n} shape={shape} index={n} />
            ))}
          </g>
        </svg>
        <div className="cv-cursor" style={{ "--x": `${cursor[0]}px`, "--y": `${cursor[1]}px` } as CSSProperties} aria-hidden="true">
          <svg className="cv-tip" viewBox="0 0 32 32" width="30" height="30" style={{ left: -hx * (30 / 32), top: -hy * (30 / 32) }}>
            {astridCursor.art}
          </svg>
          <span className="cv-follow">
            <AstridLogo expression={look?.eyes ?? "idle"} />
            <span className={`cv-orb${look ? " on" : ""}`}>
              {look && <ThinkingOrb state={look.orb} size={20} theme="dark" paused={reducedMotion()} />}
            </span>
          </span>
          {say && <span className={`cv-say${flip ? " cv-say-left" : ""}`}>{say}</span>}
        </div>
      </div>
      <header className="cv-float">
        {title && (
          <h1 className="cv-title" onPointerDown={() => native()?.postMessage({ type: "drag-start" })}>
            {title}
          </h1>
        )}
        <button type="button" className="cv-close" aria-label="Close canvas" onClick={() => native()?.postMessage({ type: "canvas-close" })}>
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path d="m4 4 8 8m0-8-8 8" />
          </svg>
        </button>
      </header>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
