import { useEffect, useRef, useState, type CSSProperties } from "react";
import { AstridCursor, SystemCursor } from "./AstridCursor";
import { EngineFigure, engineAnchors, engineSize, type Stroke } from "./EngineFigure";
import { sketchArrow, sketchEllipse } from "./sketch";

/** Virtual desktop rendered at 1000×625 and scaled to the laptop screen. */
const SCREEN = { width: 1000, height: 625 };
const FIGURE = { x: 424, y: 112, width: 530 };
const scale = FIGURE.width / engineSize.width;
const place = ([x, y]: readonly number[]): [number, number] => [
  Math.round(FIGURE.x + x * scale),
  Math.round(FIGURE.y + y * scale),
];

const steps = [
  {
    anchor: "filter",
    label: "air in",
    tag: [300, 392],
    ring: [34, 30],
    say: "Start here. Air is pulled in through the filter on the left.",
  },
  {
    anchor: "valveIn",
    label: "intake valve",
    tag: [770, 104],
    ring: [26, 40],
    say: "On the intake stroke this valve lifts. Watch the spring compress.",
  },
  {
    anchor: "plug",
    label: "spark plug",
    tag: [770, 226],
    ring: [18, 34],
    say: "At the top of compression the plug fires, once every two turns.",
  },
  {
    anchor: "fins",
    label: "cooling fins",
    tag: [330, 260],
    ring: [40, 64],
    say: "Thin fins shed the heat. More surface, cooler cylinder.",
  },
  {
    anchor: "flywheel",
    label: "flywheel",
    tag: [520, 520],
    ring: [64, 40],
    say: "The flywheel's momentum carries it through the other three strokes.",
  },
] as const;

function useScale() {
  const frame = useRef<HTMLDivElement>(null);
  const [ratio, setRatio] = useState(0.8);
  useEffect(() => {
    const element = frame.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) =>
      setRatio(entry.contentRect.width / SCREEN.width),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return { frame, ratio };
}

function MenuBar() {
  return (
    <div className="mac-menubar">
      <span className="mac-menubar-left">
        <svg viewBox="0 0 14 11" width="14" height="11" aria-hidden="true">
          <path d="M2 2h10L10 5H4Z M2 9h10L10 6H4Z" fill="#4459f9" />
        </svg>
        <b>Preview</b>
        <span>File</span>
        <span>Edit</span>
        <span>View</span>
        <span>Go</span>
        <span>Window</span>
      </span>
      <span className="mac-island">
        <i className="island-dot" />
        Astrid
        <span className="island-wave" aria-hidden="true">
          {Array.from({ length: 7 }, (_, index) => (
            <b key={index} style={{ "--i": index } as CSSProperties} />
          ))}
        </span>
      </span>
      <span className="mac-menubar-right">Wed 11 Nov&nbsp;&nbsp;9:41</span>
    </div>
  );
}

function ReaderWindow() {
  return (
    <div className="mac-window reader">
      <div className="mac-titlebar">
        <span className="traffic">
          <i />
          <i />
          <i />
        </span>
        <span>how-engines-work.pdf — Page 4 of 31</span>
      </div>
      <div className="reader-page">
        <span className="reader-kicker">CHAPTER 4</span>
        <h4>4.2 The four-stroke cycle</h4>
        {[96, 88, 93, 70].map((width, index) => (
          <i key={index} className="reader-line" style={{ width: `${width}%` }} />
        ))}
        <p className="reader-highlight">
          Each cycle needs two crank revolutions: intake, compression, power
          and exhaust.
        </p>
        {[90, 84].map((width, index) => (
          <i key={index} className="reader-line" style={{ width: `${width}%` }} />
        ))}
        <div className="reader-schematic" aria-hidden="true">
          <span />
          <span />
          <span />
          <small>Fig. 4.3 — valve timing (not to scale)</small>
        </div>
        {[92, 60].map((width, index) => (
          <i key={index} className="reader-line" style={{ width: `${width}%` }} />
        ))}
      </div>
    </div>
  );
}

export function HeroScreen() {
  const { frame, ratio } = useScale();
  const [index, setIndex] = useState(0);
  const [stroke, setStroke] = useState<{ stroke: Stroke; crank: number }>({
    stroke: "intake",
    crank: 0,
  });
  useEffect(() => {
    const timer = setInterval(() => setIndex((value) => (value + 1) % steps.length), 4600);
    return () => clearInterval(timer);
  }, []);
  const step = steps[index];
  const target = place(engineAnchors[step.anchor]);
  const arrow = sketchArrow(
    [step.tag[0] + (step.tag[0] < target[0] ? 120 : -6), step.tag[1] + 16],
    [
      target[0] + (step.tag[0] < target[0] ? -step.ring[0] - 4 : step.ring[0] + 4),
      target[1],
    ],
    step.tag[1] > target[1] ? -0.22 : 0.22,
  );

  return (
    <div className="hero-screen" ref={frame}>
      <div
        className="hero-screen-canvas"
        style={{ transform: `scale(${ratio})` }}
        aria-label="Illustration: Astrid explains an engine diagram over a PDF on a Mac"
        role="img"
      >
        <MenuBar />
        <ReaderWindow />
        <div className="figure-card">
          <div className="figure-corners" aria-hidden="true">
            <span>FIG 1 · BUILT FROM PAGE 4</span>
            <span>SINGLE-CYLINDER ENGINE</span>
            <span>ANATOMY · LIVE MODEL</span>
            <span className="figure-readout">
              {stroke.stroke.toUpperCase()} STROKE
            </span>
          </div>
        </div>
        <div
          className="figure-slot"
          style={{ left: FIGURE.x, top: FIGURE.y, width: FIGURE.width }}
        >
          <EngineFigure
            onStroke={(next, crank) => setStroke({ stroke: next, crank })}
          />
        </div>
        <svg
          className="ink-layer"
          viewBox={`0 0 ${SCREEN.width} ${SCREEN.height}`}
          aria-hidden="true"
        >
          <g key={index} className="ink-draw">
            <path
              pathLength={1}
              d={sketchEllipse(target[0], target[1], step.ring[0], step.ring[1], index + 3)}
            />
            <path pathLength={1} className="ink-delay" d={arrow.line} />
            <path pathLength={1} className="ink-delay-2" d={arrow.head} />
          </g>
        </svg>
        <span
          key={`tag-${index}`}
          className="ink-tag"
          style={{ left: step.tag[0], top: step.tag[1] }}
        >
          {step.label}
        </span>
        <SystemCursor x={252} y={330} />
        <AstridCursor x={target[0] + 6} y={target[1] + 4} speaking />
        <div className="subtitle" key={`say-${index}`}>
          <span className="subtitle-name">ASTRID</span>
          {step.say}
        </div>
      </div>
    </div>
  );
}
