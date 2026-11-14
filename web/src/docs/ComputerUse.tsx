import { useState, type CSSProperties, type ReactNode } from "react";
import { ThinkingOrb } from "thinking-orbs";
import type { OrbState } from "thinking-orbs";
import { Liquid } from "liquid-gooey";
import { HugeiconsIcon } from "@hugeicons/react";
import { ArrowUp02Icon, AudioWave01Icon } from "@hugeicons/core-free-icons";
import { AstridLogo, type EyeExpression } from "../components/AstridLogo";
import { cursors } from "../components/cursors";

const SIZE = 32;
const cursorNamed = (name: string) => cursors.find((cursor) => cursor.name === name)!;

/** Astrid on screen: the cursor, with the mark and its state orb trailing it. */
function Operator({ cursor, eyes, orb }: { cursor: string; eyes: EyeExpression; orb: OrbState }) {
  const art = cursorNamed(cursor);
  const [hx, hy] = art.hot;
  return (
    <div className="cu-operator">
      <svg
        className="cu-art"
        viewBox="0 0 32 32"
        width={SIZE}
        height={SIZE}
        style={{ left: -hx, top: -hy }}
        aria-hidden="true"
      >
        {art.art}
      </svg>
      <span className="cu-follow" style={{ left: 32 - hx + 4, top: 32 - hy - 6 }}>
        <AstridLogo expression={eyes} />
        <ThinkingOrb state={orb} size={20} theme="dark" />
      </span>
    </div>
  );
}

/** The presence step box: state line, what Astrid is doing, and the choices. */
function StepBox({
  orb,
  state,
  text,
  actions,
  hint,
}: {
  orb: OrbState;
  state: string;
  text: string;
  actions?: [string, string];
  hint?: string;
}) {
  return (
    <Liquid blur={8} contrast={20} fill="#151515" shadow="inset 0 0 0 1px #2a2a2a">
      <Liquid.Item transition="smooth">
        <div className="cu-box">
          <div className="cu-state">
            <ThinkingOrb state={orb} size={20} theme="dark" />
            <span>{state}</span>
          </div>
          <p>{text}</p>
          {actions && (
            <div className="cu-actions">
              <span className="cu-btn">{actions[0]}</span>
              <span className="cu-btn cu-btn-ghost">{actions[1]}</span>
            </div>
          )}
          {hint && <small>{hint}</small>}
        </div>
      </Liquid.Item>
    </Liquid>
  );
}

/** A small, generic app window for the cursor to work in. */
function Window({ children, title = "Report.numbers" }: { children: ReactNode; title?: string }) {
  return (
    <div className="cu-window">
      <div className="cu-titlebar">
        <i />
        <i />
        <i />
        <span>{title}</span>
      </div>
      <div className="cu-body">{children}</div>
    </div>
  );
}

type Scene = {
  id: string;
  name: string;
  note: string;
  cursor: string;
  eyes: EyeExpression;
  orb: OrbState;
  /** Where the cursor rests and where it travels to, as % of the stage. */
  from: [number, number];
  to: [number, number];
  screen: ReactNode;
  box: Parameters<typeof StepBox>[0];
};

const rows = (
  <div className="cu-rows">
    {["Revenue", "Costs", "Margin", "Headcount", "Totals"].map((label, index) => (
      <div key={label} className={index === 4 ? "cu-row cu-row-key" : "cu-row"}>
        <span>{label}</span>
        <b />
      </div>
    ))}
  </div>
);

const scenes: Scene[] = [
  {
    id: "look",
    name: "Look",
    note: "one approved frame",
    cursor: "Focus",
    eyes: "searching",
    orb: "searching",
    from: [30, 70],
    to: [62, 40],
    screen: <Window>{rows}</Window>,
    box: { orb: "searching", state: "Looking", text: "Reading your screen. One frame, nothing stored." },
  },
  {
    id: "show",
    name: "Show me",
    note: "points, never clicks",
    cursor: "Arrow click",
    eyes: "working",
    orb: "working",
    from: [24, 72],
    to: [74, 30],
    screen: (
      <Window>
        <div className="cu-toolbar">
          <span>File</span>
          <span>Edit</span>
          <span className="cu-target">Export</span>
        </div>
        {rows}
      </Window>
    ),
    box: { orb: "listening", state: "Step 2 of 4", text: "Export is up here, top right.", actions: ["Do it", "Show me"] },
  },
  {
    id: "click",
    name: "Do it · click",
    note: "after you press Do it",
    cursor: "Press button",
    eyes: "solving",
    orb: "working",
    from: [30, 74],
    to: [72, 30],
    screen: (
      <Window>
        <div className="cu-toolbar">
          <span>File</span>
          <span>Edit</span>
          <span className="cu-target cu-pressed">Export</span>
        </div>
        {rows}
        <span className="cu-ripple" aria-hidden="true" />
      </Window>
    ),
    box: { orb: "working", state: "Doing step 2", text: "Clicking Export on a fresh frame.", hint: "SAY “STOP” ANY TIME" },
  },
  {
    id: "type",
    name: "Do it · type",
    note: "short text only",
    cursor: "Text select",
    eyes: "composing",
    orb: "composing",
    from: [70, 76],
    to: [36, 44],
    screen: (
      <Window title="Save as">
        <label className="cu-field">
          <span>Name</span>
          <b>
            <em className="cu-typed">Q3 report</em>
          </b>
        </label>
        <div className="cu-field-row">
          <span className="cu-chip">PDF</span>
          <span className="cu-chip">Desktop</span>
        </div>
      </Window>
    ),
    box: { orb: "composing", state: "Doing step 3", text: "Typing “Q3 report” into Name." },
  },
  {
    id: "scroll",
    name: "Do it · scroll",
    note: "finds what’s off screen",
    cursor: "Grab",
    eyes: "searching",
    orb: "connecting",
    from: [56, 70],
    to: [56, 40],
    screen: (
      <Window>
        <div className="cu-scroll">
          {rows}
          {rows}
        </div>
      </Window>
    ),
    box: { orb: "connecting", state: "Doing step 1", text: "Scrolling down to Totals." },
  },
  {
    id: "ask",
    name: "Needs your OK",
    note: "outside the grant",
    cursor: "Not allowed",
    eyes: "idle",
    orb: "breathing",
    from: [40, 70],
    to: [70, 36],
    screen: (
      <Window title="Mail">
        <div className="cu-toolbar">
          <span>Reply</span>
          <span className="cu-target cu-blocked">Send</span>
        </div>
        <div className="cu-mail">
          <b />
          <b />
          <b />
        </div>
      </Window>
    ),
    box: { orb: "breathing", state: "Waiting for you", text: "Sending mail isn’t something I can do.", actions: ["Show me", "Stop"] },
  },
];

function SceneCard({ scene, index }: { scene: Scene; index: number }) {
  return (
    <article className="cc-card cu-card">
      <div
        className={`cc-stage cu-stage cu-scene-${scene.id}`}
        style={
          {
            "--from-x": `${scene.from[0]}%`,
            "--from-y": `${scene.from[1]}%`,
            "--to-x": `${scene.to[0]}%`,
            "--to-y": `${scene.to[1]}%`,
          } as CSSProperties
        }
      >
        {scene.screen}
        <div className="cu-pilot">
          <Operator cursor={scene.cursor} eyes={scene.eyes} orb={scene.orb} />
        </div>
        <div className="cu-box-slot">
          <StepBox {...scene.box} />
        </div>
      </div>
      <footer>
        <span>{String(index + 1).padStart(2, "0")}</span>
        <b>{scene.name}</b>
        <small>{scene.note}</small>
      </footer>
    </article>
  );
}

export function ComputerUseGrid() {
  return (
    <div className="cc-grid cu-grid">
      {scenes.map((scene, index) => (
        <SceneCard key={scene.id} scene={scene} index={index} />
      ))}
    </div>
  );
}

/** The presence ask box: mark, input, voice, and a send button that springs out. */
export function AskBox() {
  const [text, setText] = useState("");
  const ready = text.trim().length > 0;
  return (
    <div className="cu-ask">
      <Liquid blur={8} contrast={20} fill="#151515" shadow="inset 0 0 0 1px #2a2a2a">
        <div className="cu-ask-row">
          <Liquid.Item transition="bouncy">
            <form className="cu-input" onSubmit={(event) => event.preventDefault()}>
              <span className="cu-input-mark">
                <AstridLogo animated tracking />
              </span>
              <input
                value={text}
                onChange={(event) => setText(event.target.value)}
                placeholder="Ask anything, or tell me what to do on screen…"
                aria-label="Ask Astrid (demo)"
              />
              {!ready && (
                <span className="cu-mic" aria-hidden="true">
                  <HugeiconsIcon icon={AudioWave01Icon} size={18} strokeWidth={2} />
                </span>
              )}
            </form>
          </Liquid.Item>
          <Liquid.Item transition="bouncy" x={ready ? 0 : -68} scale={ready ? 1 : 0.3}>
            <button
              type="button"
              className={`cu-send${ready ? "" : " cu-send-hidden"}`}
              aria-label="Send"
              tabIndex={ready ? 0 : -1}
              onClick={() => setText("")}
            >
              <HugeiconsIcon icon={ArrowUp02Icon} size={20} strokeWidth={2} />
            </button>
          </Liquid.Item>
        </div>
      </Liquid>
    </div>
  );
}
