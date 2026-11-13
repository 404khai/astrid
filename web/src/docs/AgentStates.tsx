import { useEffect, useRef, useState } from "react";
import { ThinkingOrb } from "thinking-orbs";
import type { OrbState } from "thinking-orbs";
import { VoiceRing, useMicrophone } from "voice-ring";
import { Liquid } from "liquid-gooey";
import { AstridLogo, type EyeExpression } from "../components/AstridLogo";

/** Each orb state as Astrid uses it, named for what the person sees. */
const states: { orb: OrbState; name: string; use: string; eyes: EyeExpression }[] = [
  { orb: "working", eyes: "working", name: "Working", use: "busy on an ask" },
  { orb: "searching", eyes: "searching", name: "Searching", use: "looking at a frame" },
  { orb: "solving", eyes: "solving", name: "Solving", use: "working out the fix" },
  { orb: "listening", eyes: "listening", name: "Listening", use: "hearing you talk" },
  { orb: "connecting", eyes: "connecting", name: "Connecting", use: "reaching a device" },
  { orb: "weaving", eyes: "weaving", name: "Weaving", use: "joining sources" },
  { orb: "composing", eyes: "composing", name: "Composing", use: "writing the answer" },
  { orb: "shaping", eyes: "sketching", name: "Sketching", use: "drawing on screen" },
  { orb: "breathing", eyes: "idle", name: "Idle", use: "waiting for you" },
];

function useReducedMotion() {
  const [reduced, setReduced] = useState(
    () => window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );
  useEffect(() => {
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const change = () => setReduced(query.matches);
    query.addEventListener("change", change);
    return () => query.removeEventListener("change", change);
  }, []);
  return reduced;
}

/** The mark with its state orb beside it, the way a cursor trails the mark. */
export function OrbGrid() {
  const reduced = useReducedMotion();
  return (
    <div className="cc-grid as-grid">
      {states.map(({ orb, name, use, eyes }, index) => (
        <article key={orb} className="cc-card">
          <div className="cc-stage as-stage">
            <div className="as-pair">
              <AstridLogo expression={eyes} />
              <ThinkingOrb state={orb} size={64} theme="dark" paused={reduced} />
            </div>
          </div>
          <footer>
            <span>{String(index + 1).padStart(2, "0")}</span>
            <b>{name}</b>
            <small>{use}</small>
          </footer>
        </article>
      ))}
    </div>
  );
}

/** Push-to-talk: a synthetic voice by default, the real mic on request. */
export function ListeningCard() {
  const reduced = useReducedMotion();
  const mic = useMicrophone();
  const live = mic.state === "live";
  const synthetic = () => {
    const t = performance.now() / 1000;
    return Math.max(0, 0.35 + 0.3 * Math.sin(t * 5.1) * Math.sin(t * 1.7));
  };
  return (
    <div className="as-listen">
      <div className="as-listen-ring">
        <VoiceRing
          size={200}
          theme="dark"
          state="listening"
          stream={live ? mic.stream : null}
          level={live || reduced ? undefined : synthetic}
        />
        <span className="as-listen-mark">
          <AstridLogo expression="listening" />
        </span>
      </div>
      <div className="as-listen-copy">
        <span className="eyebrow">LISTENING · VOICE RING</span>
        <p>
          {live
            ? "Following your microphone. Audio stays in this page."
            : "A simulated voice. Hold to talk, let go to send."}
        </p>
        {mic.supported && (
          <button type="button" className="as-button" onClick={live ? mic.stop : mic.start}>
            {live ? "Stop mic" : "Try with your mic"}
          </button>
        )}
      </div>
    </div>
  );
}

const answer = [
  { kind: "title", text: "Your keys don’t match types." },
  { kind: "line", text: "Column B holds “1042” as text; the lookup searches for 1042 as a number." },
  { kind: "step", text: "Select B2:B240" },
  { kind: "step", text: "Data → Convert to number" },
  { kind: "step", text: "Re-run the lookup in D2" },
  { kind: "meta", text: "SAW 1 FRAME · POINTED AT B2 · NOTHING STORED" },
] as const;

const phases: { orb: OrbState; eyes: EyeExpression; label: string }[] = [
  { orb: "searching", eyes: "searching", label: "Looking" },
  { orb: "solving", eyes: "solving", label: "Solving" },
  { orb: "composing", eyes: "composing", label: "Writing" },
];

/** A liquid output surface: the status pill and the answer box share one goo. */
export function OutputBox() {
  const reduced = useReducedMotion();
  const [phase, setPhase] = useState(reduced ? 2 : 0);
  const [written, setWritten] = useState(reduced ? answer.length : 0);
  const root = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const observer = new IntersectionObserver(([entry]) => setVisible(entry.isIntersecting));
    if (root.current) observer.observe(root.current);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    if (reduced || !visible) return;
    const writing = phase === 2;
    const delay = writing ? (written < answer.length ? 700 : 4200) : 1800;
    const timer = window.setTimeout(() => {
      if (!writing) setPhase(phase + 1);
      else if (written < answer.length) setWritten(written + 1);
      else {
        setPhase(0);
        setWritten(0);
      }
    }, delay);
    return () => window.clearTimeout(timer);
  }, [phase, written, reduced, visible]);
  const current = phases[phase];
  let step = 0;
  return (
    <div className="as-output" ref={root}>
      <Liquid blur={8} contrast={20} fill="#181818" shadow="inset 0 0 0 1px #292929">
        <Liquid.Item x={phase * 16} transition="bouncy">
          <div className="as-pill">
            <AstridLogo expression={current.eyes} />
            <ThinkingOrb state={current.orb} size={20} theme="dark" paused={reduced} />
            <span>{current.label}</span>
          </div>
        </Liquid.Item>
        <Liquid.Item transition="smooth" delay={60}>
          <div className="as-box" aria-live="polite">
            <p className="as-ask">“Why won’t my VLOOKUP find anything?”</p>
            {answer.slice(0, written).map((row, index) => (
              <p key={index} className={`as-${row.kind}`}>
                {row.kind === "step" && <span>{++step}</span>}
                {row.text}
              </p>
            ))}
            {phase === 2 && written < answer.length && <i className="as-caret" aria-hidden="true" />}
          </div>
        </Liquid.Item>
      </Liquid>
    </div>
  );
}
