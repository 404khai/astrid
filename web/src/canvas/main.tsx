import { StrictMode, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource/space-grotesk/latin-400.css";
import "@fontsource/space-grotesk/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import { ThinkingOrb } from "thinking-orbs";
import type { OrbState } from "thinking-orbs";
import { AstridLogo, type EyeExpression } from "../components/AstridLogo";
import { astridCursor } from "../components/cursors";
import { anchor, glowStyle, motionStyle, palette, ShapeView, type Shape } from "./shapes";
import { defaultCamera, flatten, ISO_CSS, paintOrder, project, rotorSvg, solid, type Camera } from "./solids";
import { demo } from "./demo";
import { applyEdit, driven, fold, liveValues, placed, visible } from "./interact";
import { Toolbar } from "./Toolbar";
import { drawingPng, drawingSvg, fileName } from "./exporter";
import "./canvas.css";

/** Astrid's live canvas (ADR 0036). The presence app shows this page in its
 * own panel and relays the runtime's validated canvas events to it, plus the
 * ask surface's state so a follow-up shows on the canvas too. */
type Incoming =
  | { type: "canvas"; event: "begin"; width: number; height: number }
  | { type: "canvas"; event: "shape"; shape: Shape }
  | { type: "canvas"; event: "end" | "cancel" }
  | { type: "canvas"; event: "replay"; shapes: Shape[] }
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

/** The desktop app's thread embeds this page per diagram (ADR 0037): no close
 * button or window drag, and a saved drawing can be replayed at once. */
const embed = new URLSearchParams(location.search).has("embed");
/** A gallery thumbnail: drawing only, no controls. */
const thumb = new URLSearchParams(location.search).has("thumb");
if (thumb) document.documentElement.classList.add("cv-thumb");
if (embed) document.documentElement.classList.add("cv-embedded");

const reducedMotion = () => matchMedia("(prefers-reduced-motion: reduce)").matches;


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

/** The sheet Astrid draws on (Astrid Weaver): its colour, whether it is
 * light enough to need dark ink, and the dot grid. */
type Sheet = { background: string | null; light: boolean; grid: boolean };
const plainSheet: Sheet = { background: null, light: false, grid: true };
function sheetOf(s: Shape): Sheet {
  const c = s.background && s.background !== "none" ? palette[s.background] ?? s.background : null;
  return { background: c, light: c ? luminance(c) > 0.45 : false, grid: s.grid ?? true };
}
/** Relative luminance of a validated #hex colour (0 dark … 1 light). */
function luminance(hex: string): number {
  let h = hex.replace("#", "");
  if (h.length <= 4) h = [...h].map((c) => c + c).join("");
  const [r, g, b] = [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** A spinning rotor redraws itself each frame outside React, so a turning
 * fan costs one path rebuild per frame and nothing else re-renders. */
/** The toolbar's play/pause and tempo, read by rotors every frame. */
const clock = { tempo: 1, paused: false };

function SpinningRotor({ shape, cam }: { shape: Shape; cam: Camera }) {
  const ref = useRef<SVGGElement>(null);
  const angle = useRef(0);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const speed = shape.speed ?? 1;
    const turn = (Math.PI * 2) / 3.2;
    if (reducedMotion()) {
      el.innerHTML = rotorSvg(shape, cam, angle.current);
      return;
    }
    let frame = 0;
    let last = performance.now();
    const tick = (now: number) => {
      if (!clock.paused) angle.current += ((now - last) / 1000) * turn * speed * clock.tempo;
      last = now;
      el.innerHTML = rotorSvg(shape, cam, angle.current);
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [shape, cam]);
  return <g ref={ref} />;
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
  const [sheet, setSheet] = useState<Sheet>(plainSheet);
  // Astrid Weaver interactions.
  const [explode, setExplode] = useState(0);
  const [cut, setCut] = useState(false);
  const [paused, setPaused] = useState(false);
  const [tempo, setTempo] = useState(1);
  const [overrides, setOverrides] = useState<Record<string, number>>({});
  const [stepAt, setStepAt] = useState<number | null>(null);
  const [hovered, setHovered] = useState<{ id: string; at: [number, number] } | null>(null);
  const [tapped, setTapped] = useState<string | null>(null);
  clock.tempo = tempo;
  clock.paused = paused;
  /** A new drawing clears the old one at its first real shape, so a reply of
   * only edits changes the drawing already there. */
  const fresh = useRef(false);
  const latest = useRef<Shape[]>([]);
  /** Whether this reply drew something new (not only edits): only a new
   * drawing starts its walkthrough by itself. */
  const drewNew = useRef(false);
  const queue = useRef<Shape[]>([]);
  const ended = useRef(true);
  const touched = useRef(false);
  /** Pointers down on the stage, for a two-finger pinch. */
  const fingers = useRef(new Map<number, [number, number]>());
  const timer = useRef<number | undefined>(undefined);
  const stage = useRef<HTMLDivElement>(null);
  const content = useRef<SVGGElement>(null);

  // Built from the raw shapes for the current camera, so turning the figure
  // re-projects every solid, label and arrow. Solids are drawn back to front.
  latest.current = drawn;
  const values = useMemo(() => liveValues(drawn, overrides), [drawn, overrides]);
  const steps = useMemo(() => drawn.filter((s) => s.shape === "step"), [drawn]);
  const scene = useMemo(() => {
    // What is on the canvas now: hidden parts and cut-away casings left out,
    // exploded parts moved, live values shown and driving their parts' speed.
    const shown = drawn
      .map((shape, n) => ({ shape, n }))
      .filter(({ shape }) => visible(shape) && !(cut && shape.shell))
      .map(({ shape, n }) => {
        let s = placed(shape, explode);
        const live = values.find((v) => v.id === s.id && s.shape === "value");
        if (live) s = { ...s, value: live.now };
        const factor = driven(s, values);
        if (s.motion && factor !== 1) s = { ...s, speed: (s.speed ?? 1) * factor };
        return { shape: s, n };
      });
    const solids = paintOrder(shown
      .map(({ shape, n }) => ({ shape, n, built: solid(shape, cam) }))
      .filter((s) => s.built)
      .map((s) => ({ ...s, built: s.built! }))
    );
    const flats = shown.map(({ shape, n }) => ({ shape: flatten(shape, cam), n })).filter((s) => !solid(s.shape, cam));
    return { solids, flats, last: drawn.length - 1, motion: shown.some((s) => s.shape.motion) };
  }, [drawn, cam, explode, cut, values]);

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

  /** Zooms to the parts with an id (an edit's focus). */
  const focusOn = (id: string) => {
    const parts = content.current?.querySelectorAll<SVGGraphicsElement>(`[data-part="${CSS.escape(id)}"]`);
    if (!parts?.length) return;
    let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
    parts.forEach((p) => {
      const b = p.getBBox();
      [x0, y0, x1, y1] = [Math.min(x0, b.x), Math.min(y0, b.y), Math.max(x1, b.x + b.width), Math.max(y1, b.y + b.height)];
    });
    const zoom = Math.min((W * 0.5) / Math.max(x1 - x0, 1), (H * 0.5) / Math.max(y1 - y0, 1), 1.8);
    touched.current = true;
    setTapped(id);
    setView({ zoom, x: W / 2 - ((x0 + x1) / 2) * zoom, y: H / 2 + 24 - ((y0 + y1) / 2) * zoom });
  };

  /** Clears the canvas for a new drawing. */
  const clear = () => {
    touched.current = false;
    setTitle(null);
    setDrawn([]);
    setSay(null);
    setSheet(plainSheet);
    setCam(defaultCamera);
    setView(home);
    setExplode(0);
    setCut(false);
    setOverrides({});
    setStepAt(null);
    setTapped(null);
  };

  /** Shows one step of a walkthrough: the cursor goes to it, its part lights. */
  const goStep = (i: number | null) => {
    const list = latest.current.filter((s) => s.shape === "step");
    if (i === null || !list[i]) {
      setStepAt(null);
      setSay(null);
      setTapped(null);
      return;
    }
    const s = list[i];
    setStepAt(i);
    setSay(s.text ?? null);
    setTapped(typeof s.focus === "string" ? s.focus : null);
    if (s.x !== undefined) setTarget(s.z !== undefined ? { world: [s.x, s.y!, s.z] } : { flat: [s.x, s.y!] });
  };
  const stepRef = useRef(stepAt);
  stepRef.current = stepAt;
  const goStepRef = useRef(goStep);
  goStepRef.current = goStep;

  // One shape at a time, so the drawing reads as being made: the cursor
  // travels to each shape, then it is drawn there.
  const pump = () => {
    if (timer.current) return;
    const next = queue.current.shift();
    if (!next) {
      setDrawing(ended.current ? null : "weaving");
      if (ended.current && !touched.current) requestAnimationFrame(() => fit(false));
      // A drawing that explains in steps starts walking through them.
      if (ended.current && drewNew.current && latest.current.some((s) => s.shape === "step")) {
        drewNew.current = false;
        window.setTimeout(() => goStepRef.current(0), 700);
      }
      return;
    }
    if (fresh.current) {
      fresh.current = false;
      drewNew.current = next.shape !== "edit";
      if (drewNew.current) clear();
    }
    setDrawing("sketching");
    if (next.shape === "edit") {
      if (next.value !== undefined) setOverrides(({ [next.id!]: _, ...rest }) => rest);
      setDrawn((list) => applyEdit(list, next));
      if (next.focus === true) requestAnimationFrame(() => requestAnimationFrame(() => focusOn(next.id!)));
      timer.current = window.setTimeout(step, 220);
      return;
    }
    if (next.shape === "title" || next.shape === "canvas") {
      if (next.shape === "title") setTitle(next.text ?? null);
      else setSheet(sheetOf(next));
      timer.current = window.setTimeout(step, 120);
      return;
    }
    const built = solid(next);
    if (built) setTarget({ world: built.world });
    else if (next.shape === "point" && next.z !== undefined) setTarget({ world: [next.x!, next.y!, next.z] });
    else if ((next.shape === "text" || next.shape === "callout") && next.z !== undefined) setTarget({ world: [next.x!, next.y!, next.z] });
    else if (next.points?.length && (next.points[next.points.length - 1] as number[]).length === 3)
      setTarget({ world: next.points[next.points.length - 1] as unknown as [number, number, number] });
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
          fresh.current = true;
          setSay(null);
          setStepAt(null);
          setDrawing("composing");
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
        case "replay": {
          // A saved drawing: everything at once, framed, with no cursor tour.
          window.clearTimeout(timer.current);
          timer.current = undefined;
          queue.current = [];
          ended.current = true;
          fresh.current = false;
          const titled = m.shapes.find((s) => s.shape === "title");
          clear();
          setTitle(titled?.text ?? null);
          const painted = [...m.shapes].reverse().find((s) => s.shape === "canvas");
          setSheet(painted ? sheetOf(painted) : plainSheet);
          setDrawn(fold(m.shapes.filter((s) => s.shape !== "title" && s.shape !== "point" && s.shape !== "canvas")));
          setSay(null);
          setDrawing(null);
          setCam(defaultCamera);
          setView(home);
          requestAnimationFrame(() => requestAnimationFrame(() => fit(false)));
          break;
        }
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
  // pinch zooms, double-click puts it back. On a touch screen (the phone app,
  // ADR 0039) two fingers pinch to zoom.
  const onPointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0 || (e.target as Element).closest("button, .cv-title")) return;
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    touched.current = true;
    setMoving(true);
    fingers.current.set(e.pointerId, [e.clientX, e.clientY]);
    let last = { x: e.clientX, y: e.clientY };
    const start = last;
    const part = (e.target as Element).closest<SVGElement>("[data-part]")?.dataset.part;
    const turn = hasSolids && !e.shiftKey;
    const move = (ev: PointerEvent) => {
      if (ev.pointerId !== e.pointerId) return;
      const before = [...fingers.current.values()];
      fingers.current.set(ev.pointerId, [ev.clientX, ev.clientY]);
      if (fingers.current.size >= 2) {
        const after = [...fingers.current.values()];
        const span = (p: [number, number][]) => Math.hypot(p[0][0] - p[1][0], p[0][1] - p[1][1]);
        const ratio = span(after) / Math.max(span(before), 1);
        const rect = stage.current!.getBoundingClientRect();
        const px = ((after[0][0] + after[1][0]) / 2 - rect.left - offset.x) / unit;
        const py = ((after[0][1] + after[1][1]) / 2 - rect.top - offset.y) / unit;
        setView((v) => {
          const zoom = Math.min(6, Math.max(0.2, v.zoom * ratio));
          return { zoom, x: px - ((px - v.x) / v.zoom) * zoom, y: py - ((py - v.y) / v.zoom) * zoom };
        });
        last = { x: ev.clientX, y: ev.clientY };
        return;
      }
      const dx = ev.clientX - last.x, dy = ev.clientY - last.y;
      last = { x: ev.clientX, y: ev.clientY };
      if (turn) {
        setCam((c) => ({ azimuth: (c.azimuth + dx * 0.4 + 360) % 360, elevation: Math.min(75, Math.max(8, c.elevation + dy * 0.25)) }));
      } else {
        setView((v) => ({ ...v, x: v.x + dx / unit, y: v.y + dy / unit }));
      }
    };
    const up = (ev: PointerEvent) => {
      if (ev.pointerId !== e.pointerId) return;
      fingers.current.delete(e.pointerId);
      if (!fingers.current.size) setMoving(false);
      // A click that didn't drag, on a part with an id, asks about it.
      if (part && Math.hypot(ev.clientX - start.x, ev.clientY - start.y) < 4) tap(part);
      removeEventListener("pointermove", move);
      removeEventListener("pointerup", up);
      removeEventListener("pointercancel", up);
    };
    addEventListener("pointermove", move);
    addEventListener("pointerup", up);
    addEventListener("pointercancel", up);
  };
  const onWheel = (e: React.WheelEvent) => {
    // Embedded in a scrolling thread, plain scrolling belongs to the thread.
    if (embed && !e.ctrlKey) return;
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
  /** Tapping a part lights it and asks Astrid about it in this conversation. */
  const tap = (id: string) => {
    setTapped(id);
    const part = latest.current.find((s) => s.id === id);
    const name = part?.label ?? part?.title ?? id;
    native()?.postMessage({ type: "ask", text: `Tell me about the ${name}${title ? ` in the ${title}` : ""}.`, part: id });
  };
  const onHover = (e: React.PointerEvent) => {
    if (moving || e.pointerType === "touch") return;
    const id = (e.target as Element).closest<SVGElement>("[data-part]")?.dataset.part;
    const rect = stage.current!.getBoundingClientRect();
    setHovered(id ? { id, at: [e.clientX - rect.left, e.clientY - rect.top] } : null);
  };

  const exportAs = async (format: "svg" | "png") => {
    const svg = stage.current?.querySelector<SVGSVGElement>(".cv-svg");
    const card = stage.current?.parentElement;
    if (!svg || !card) return;
    const background = getComputedStyle(card).backgroundColor;
    const text = drawingSvg(svg, background, W, H);
    const data = format === "svg" ? text : await drawingPng(text, W, H);
    const name = `${fileName(title)}.${format}`;
    if (native()) {
      native()!.postMessage({ type: "export", format, name, data });
      return;
    }
    const link = document.createElement("a");
    link.href = format === "svg" ? URL.createObjectURL(new Blob([data], { type: "image/svg+xml" })) : `data:image/png;base64,${data}`;
    link.download = name;
    link.click();
  };

  // Arrow keys walk the steps; space pauses the motion.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.target as Element).closest?.("input")) return;
      const count = latest.current.filter((s) => s.shape === "step").length;
      if (e.key === "ArrowRight" && count) goStepRef.current(Math.min(count - 1, (stepRef.current ?? -1) + 1));
      else if (e.key === "ArrowLeft" && count) goStepRef.current(Math.max(0, (stepRef.current ?? 1) - 1));
      else if (e.key === "Escape") goStepRef.current(null);
      else if (e.key === " ") setPaused((p) => !p);
      else return;
      e.preventDefault();
    };
    addEventListener("keydown", onKey);
    return () => removeEventListener("keydown", onKey);
  }, []);

  const reset = () => {
    touched.current = false;
    setCam(defaultCamera);
    requestAnimationFrame(() => fit(false));
  };

  const lit = hovered?.id ?? tapped;
  const highlighting = !!lit && drawn.some((s) => s.id === lit && visible(s));
  const partOf = (shape: Shape) => (shape.id ? { "data-part": shape.id } : {});
  const litClass = (shape: Shape) => (shape.id ? ` cv-part${shape.id === lit ? " cv-lit" : ""}` : " cv-part");
  const hoverName = hovered && (() => {
    const part = drawn.find((s) => s.id === hovered.id);
    return part?.label ?? part?.title ?? hovered.id;
  })();

  const activity = drawing ?? external;
  const look = activity ? looks[activity] : null;
  const [hx, hy] = astridCursor.hot;
  const flip = cursor[0] > size.w - 300;
  return (
    <main
      className={`cv-card${moving ? " cv-moving" : ""}${embed && !drawing && stepAt === null ? " cv-at-rest" : ""}${sheet.light ? " cv-light" : ""}${sheet.grid ? "" : " cv-no-grid"}${highlighting ? " cv-highlighting" : ""}${paused ? " cv-paused" : ""}`}
      style={{ ...(sheet.background ? { "--sheet": sheet.background } : {}), "--tempo": tempo } as CSSProperties}
    >
      <style>{ISO_CSS}</style>
      <div className="cv-stage" ref={stage} onPointerDown={onPointerDown} onPointerMove={onHover} onPointerLeave={() => setHovered(null)} onWheel={onWheel} onDoubleClick={reset}>
        <svg viewBox={`0 0 ${W} ${H}`} className="cv-svg" role="img" aria-label={title ?? "Astrid's drawing"}>
          <g ref={content} className="cv-view" style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.zoom})` }}>
            <g className="iso cv-solids" data-theme={sheet.light ? "light" : "dark"}>
              {scene.solids.map(({ shape, n, built }) => {
                const moving = shape.shape === "rotor" ? null : motionStyle(shape);
                const spin = shape.shape === "rotor" && shape.motion === "spin";
                return (
                  <g
                    key={n}
                    {...partOf(shape)}
                    className={`cv-solid3${n === scene.last ? " cv-new" : ""}${shape.glow ? " cv-glow" : ""}${litClass(shape)}`}
                    style={{ ...tint(shape.fill), ...glowStyle(shape) }}
                  >
                    <g className={moving?.className} style={moving?.style}>
                      {spin ? <SpinningRotor shape={shape} cam={cam} /> : <g dangerouslySetInnerHTML={{ __html: built.svg }} />}
                    </g>
                  </g>
                );
              })}
              {scene.solids.map(({ shape, n, built }) =>
                shape.label ? (
                  <text key={`l${n}`} className="cv-label" x={built.labelAt[0]} y={built.labelAt[1]} textAnchor="middle">
                    {shape.label}
                  </text>
                ) : null,
              )}
            </g>
            {scene.flats.map(({ shape, n }) => (
              <g key={n} {...partOf(shape)} className={litClass(shape).trim()}>
                <ShapeView shape={shape} index={n} />
              </g>
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
      {hovered && hoverName && !moving && (
        <span className="cv-hover" style={{ left: hovered.at[0] + 14, top: hovered.at[1] + 18 }}>
          {hoverName}
        </span>
      )}
      {!thumb && !drawing && drawn.some(visible) && (
        <Toolbar
          motion={scene.motion && !reducedMotion()}
          paused={paused}
          onPause={() => setPaused((p) => !p)}
          tempo={tempo}
          onTempo={() => setTempo((t) => ({ 0.25: 0.5, 0.5: 1, 1: 2, 2: 0.25 })[t as 0.25 | 0.5 | 1 | 2] ?? 1)}
          steps={steps.length}
          step={stepAt}
          onStep={goStep}
          explodable={drawn.some((s) => s.explode && visible(s))}
          explode={explode}
          onExplode={(t) => {
            touched.current = true;
            setExplode(t);
          }}
          shelled={drawn.some((s) => s.shell && visible(s))}
          cut={cut}
          onCut={() => setCut((c) => !c)}
          values={values}
          onValue={(id, v) => setOverrides((o) => ({ ...o, [id]: v }))}
          exportable={embed || !native()}
          onExport={exportAs}
        />
      )}
      <header className="cv-float">
        {title && (
          <h1 className="cv-title" onPointerDown={() => !embed && native()?.postMessage({ type: "drag-start" })}>
            {title}
          </h1>
        )}
        {!embed && (
          <button type="button" className="cv-close" aria-label="Close canvas" onClick={() => native()?.postMessage({ type: "canvas-close" })}>
            <svg viewBox="0 0 16 16" width="14" height="14">
              <path d="m4 4 8 8m0-8-8 8" />
            </svg>
          </button>
        )}
      </header>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
