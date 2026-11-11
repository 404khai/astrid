import { useEffect, useRef, useState } from "react";
import { boardFigure as F } from "../generated/board-figure";
import { AstridCursor } from "./AstridCursor";
import { AstridLogo } from "./AstridLogo";
import { VerseText } from "./VerseText";

const PALETTE = [
  "#1a32ec", "#8793ff", "#ec6a8a", "#30a46c", "#f7c600", "#6575ff",
  "#c53b4a", "#e5484d", "#f08a9a", "#f4a3ae", "#d92d3a", "#4459f9", "#6575ff",
];
const DNA_STOPS = [3, 9, 15, 20];
const HEART_STOPS = [1, 0, 3, 2, 4, 5, 6];
const STOP_S = 2.3;
const MORPH_S = 2.6;
const T_DNA = DNA_STOPS.length * STOP_S;
const T_HEART = HEART_STOPS.length * STOP_S;
const CYCLE = T_DNA + MORPH_S + T_HEART + MORPH_S;

const P = F.projection;
const RAD = Math.PI / 180;
const cam = {
  sinA: Math.sin(P.azimuth * RAD),
  cosA: Math.cos(P.azimuth * RAD),
  sinE: Math.sin(P.elevation * RAD),
  cosE: Math.cos(P.elevation * RAD),
  k: P.scale * Math.sqrt(1.6),
};
/** The Anatomy kit's isometric projection, so particles share the stand's camera. */
const project = (x: number, y: number, z: number) => [
  P.origin[0] + (x * cam.sinA - y * cam.cosA) * cam.k,
  P.origin[1] + ((x * cam.cosA + y * cam.sinA) * cam.sinE - z * cam.cosE) * cam.k,
  (x * cam.cosA + y * cam.sinA) * cam.cosE + z * cam.sinE,
];
const ease = (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);
const noise = (i: number) => {
  const v = Math.sin(i * 91.37) * 43758.5453;
  return v - Math.floor(v);
};

type Stage =
  | { kind: "dna"; stop: number }
  | { kind: "heart"; stop: number }
  | { kind: "morph"; toHeart: boolean };

function stageAt(t: number): { stage: Stage; morph: number } {
  if (t < T_DNA) return { stage: { kind: "dna", stop: Math.floor(t / STOP_S) }, morph: 0 };
  t -= T_DNA;
  if (t < MORPH_S) return { stage: { kind: "morph", toHeart: true }, morph: ease(t / MORPH_S) };
  t -= MORPH_S;
  if (t < T_HEART) return { stage: { kind: "heart", stop: Math.floor(t / STOP_S) }, morph: 1 };
  t -= T_HEART;
  return { stage: { kind: "morph", toHeart: false }, morph: 1 - ease(t / MORPH_S) };
}

/** Particles on the Anatomy stand: a turning helix that dissolves and
 * rebuilds itself as a beating heart, then back. One clock drives the
 * particles, the pointer and the captions. */
function ParticleBoard({ onStage }: { onStage: (stage: Stage) => void }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const pointer = useRef<HTMLDivElement>(null);
  const report = useRef(onStage);
  report.current = onStage;
  useEffect(() => {
    const element = canvas.current;
    const context = element?.getContext("2d");
    if (!element || !context) return;
    const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const ratio = Math.min(2, devicePixelRatio || 1);
    element.width = F.width * ratio;
    element.height = F.height * ratio;
    const n = F.count;
    const xs = new Float32Array(n);
    const ys = new Float32Array(n);
    const ds = new Float32Array(n);
    const cs = new Uint8Array(n);
    const order = new Uint16Array(n);
    let frame = 0;
    let visible = false;
    let start = 0;
    let shown = "";
    const draw = (seconds: number) => {
      const t = seconds % CYCLE;
      const { stage, morph } = stageAt(t);
      const spin = seconds * 0.7;
      // Turn the heart's front (its x–z plane) to face the iso camera, then sway.
      const sway = -Math.PI / 4 + Math.sin(seconds * 0.5) * 0.35;
      const beat = 1 + 0.045 * Math.max(0, Math.sin(seconds * 7.5)) ** 6;
      const [hx, hy, hz] = F.heartCentre;
      const cd = Math.cos(spin), sd = Math.sin(spin), ch = Math.cos(sway), sh = Math.sin(sway);
      for (let i = 0; i < n; i++) {
        const o = i * 5;
        const ax = F.dna[o], ay = F.dna[o + 1], az = F.dna[o + 2];
        const dx = ax * cd - ay * sd, dy = ax * sd + ay * cd;
        let bx = F.heart[o] * beat, by = F.heart[o + 1] * beat;
        const bz = hz + (F.heart[o + 2] - hz) * beat;
        const rx = bx * ch - by * sh;
        by = bx * sh + by * ch;
        bx = rx + hx;
        by += hy;
        const burst = Math.sin(Math.PI * morph) * (40 + noise(i) * 70);
        const angle = noise(i + 7) * Math.PI * 2;
        const x = dx + (bx - dx) * morph + Math.cos(angle) * burst;
        const y = dy + (by - dy) * morph + Math.sin(angle) * burst;
        const z = az + (bz - az) * morph + (noise(i + 3) - 0.3) * burst;
        const [px, py, depth] = project(x, y, z);
        xs[i] = px;
        ys[i] = py;
        ds[i] = depth;
        cs[i] = morph < 0.5 ? F.dna[o + 3] : F.heart[o + 3];
        order[i] = i;
      }
      order.sort((a, b) => ds[a] - ds[b]);
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      context.clearRect(0, 0, F.width, F.height);
      for (let j = 0; j < n; j++) {
        const i = order[j];
        const shade = Math.max(0.35, Math.min(1, 0.6 + ds[i] / 160));
        context.globalAlpha = shade;
        context.fillStyle = PALETTE[cs[i]];
        context.beginPath();
        context.arc(xs[i], ys[i], morph > 0.05 && morph < 0.95 ? 1.6 : 2.1, 0, Math.PI * 2);
        context.fill();
      }
      context.globalAlpha = 1;
      let target: number[] | null = null;
      if (stage.kind === "dna") target = project(0, 0, F.rungZ[DNA_STOPS[stage.stop]]);
      if (stage.kind === "heart") {
        const [cx, cy, cz] = F.parts[HEART_STOPS[stage.stop]].centre;
        target = project(cx * ch - cy * sh + hx, cx * sh + cy * ch + hy, cz);
      }
      const box = pointer.current;
      const cursor = box?.firstElementChild as HTMLElement | null;
      if (box && cursor) {
        box.style.opacity = target ? "1" : "0";
        if (target) {
          cursor.style.setProperty("--x", `${(target[0] / F.width) * element.clientWidth}px`);
          cursor.style.setProperty("--y", `${(target[1] / F.height) * element.clientHeight}px`);
        }
      }
      const key = JSON.stringify(stage);
      if (key !== shown) {
        shown = key;
        report.current(stage);
      }
    };
    const tick = (now: number) => {
      if (!start) start = now;
      draw((now - start) / 1000);
      frame = visible ? requestAnimationFrame(tick) : 0;
    };
    const observer = new IntersectionObserver(([entry]) => {
      visible = entry.isIntersecting;
      if (visible && !frame && !still) frame = requestAnimationFrame(tick);
    });
    observer.observe(element);
    draw(still ? 1 : 0);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, []);
  return (
    <div className="board-figure">
      <div className="iso board-stand" data-theme="light" dangerouslySetInnerHTML={{ __html: F.svg }} />
      <canvas ref={canvas} className="board-particles" aria-hidden="true" />
      <div ref={pointer} className="board-pointer">
        <AstridCursor x={0} y={0} stick />
      </div>
    </div>
  );
}

const PAIRS = ["A–T", "G–C", "T–A", "C–G"];
const DNA_NOTES = [
  ["base pair · A–T", "This rung is an A–T base pair. A only ever pairs with T."],
  ["base pair · G–C", "G pairs with C, with three hydrogen bonds, so these rungs hold tighter."],
  ["backbone", "The twisting rails are sugar and phosphate; the rungs carry the code."],
  ["one turn", "About ten base pairs make one full turn of the helix."],
];
void PAIRS;

/** Poster-style section: blue blocks around a light panel, with Astrid holding
 * a pointer stick over the helix like a teacher at a whiteboard. */
export function DnaExplainer() {
  const [stage, setStage] = useState<Stage>({ kind: "dna", stop: 0 });
  let label = "";
  let say = "";
  if (stage.kind === "dna") [label, say] = DNA_NOTES[stage.stop];
  if (stage.kind === "heart") {
    const part = F.parts[HEART_STOPS[stage.stop]];
    label = part.name;
    say = part.detail;
  }
  if (stage.kind === "morph")
    say = stage.toHeart
      ? "Let me take it apart and build something else: a heart."
      : "And back again, from the heart to the code that builds it.";
  return (
    <div className="poster">
      <div className="poster-blocks" aria-hidden="true">
        <i className="pb pb1" />
        <i className="pb pb2" />
        <i className="pb pb3" />
        <i className="pb pb4" />
        <i className="pb pb5" />
        <i className="pb pb6" />
        <span className="halftone h1" />
        <span className="halftone h2" />
        <span className="coord c1">
          <b>S02</b>
          <em>C1</em>
          <em>04.4,03.1</em>
        </span>
        <span className="coord c2">
          <b>FIG</b>
          <em>C3</em>
          <em>05.1,08.4</em>
        </span>
        <span className="coord c3">5859</span>
      </div>
      <div className="poster-panel">
        <div className="poster-mark">
          <AstridLogo />
          <span>ASTRID</span>
        </div>
        <div className="poster-title" role="heading" aria-level={2}>
          <VerseText text={"A teacher that\npoints at the board."} tone="light" idle={false} seed={77} />
        </div>
        <div className="poster-board">
          <div className="board-head">
            <span className="board-label">
              WHITEBOARD · {stage.kind === "heart" || (stage.kind === "morph" && stage.toHeart) ? "HUMAN HEART" : "DNA"}, LIVE
            </span>
            {label && (
              <span key={label} className="board-note">
                {label}
              </span>
            )}
          </div>
          <ParticleBoard onStage={setStage} />
          <p className="board-caption" key={say}>
            <b>ASTRID</b> {say}
          </p>
        </div>
        <p className="poster-copy">
          Ask how something works and Astrid steps up to the board: a pointer in
          hand, the model taking itself apart and rebuilding into the next idea.
        </p>
      </div>
    </div>
  );
}
