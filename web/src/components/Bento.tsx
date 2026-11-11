import { useEffect, useRef, useState, type ReactNode } from "react";
import { AstridCursor } from "./AstridCursor";
import { AstridLogo } from "./AstridLogo";
import { EngineFigure } from "./EngineFigure";
import { PhoneMockupCard } from "./PhoneMockup";
import { sketchArrow, sketchEllipse } from "./sketch";

export function useInView<T extends Element>(margin = "-15% 0px") {
  const ref = useRef<T>(null);
  const [seen, setSeen] = useState(false);
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setSeen(true);
          observer.disconnect();
        }
      },
      { rootMargin: margin },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [margin]);
  return { ref, seen };
}

function Tile({
  className,
  index,
  title,
  children,
  visual,
  status,
}: {
  className: string;
  index: string;
  title: string;
  children: ReactNode;
  visual: ReactNode;
  status?: string;
}) {
  return (
    <article className={`tile ${className}`}>
      <div className="tile-visual">{visual}</div>
      <div className="tile-copy">
        <div className="tile-meta">
          <span>{index}</span>
          {status && <span className="tile-status">{status}</span>}
        </div>
        <h3>{title}</h3>
        <p>{children}</p>
      </div>
    </article>
  );
}

function PointVisual() {
  return (
    <div className="point-visual">
      <div className="mini-window">
        <div className="mac-titlebar">
          <span className="traffic">
            <i />
            <i />
            <i />
          </span>
          <span>Project settings</span>
        </div>
        <div className="mini-body">
          <nav aria-hidden="true">
            {["General", "Domains", "Git", "Environment", "Deployments"].map(
              (item) => (
                <span key={item} className={item === "Domains" ? "active" : ""}>
                  {item}
                </span>
              ),
            )}
          </nav>
          <div className="mini-panel">
            <span className="mini-heading">Domains</span>
            <span className="mini-sub">Point a domain at this project.</span>
            <div className="mini-field">
              <span>astrid.dev</span>
              <b className="mini-add">
                Add
                <span className="point-ring" aria-hidden="true" />
                <AstridCursor x={0} y={0} className="point-cursor" />
              </b>
            </div>
            <i className="reader-line" style={{ width: "70%" }} />
            <i className="reader-line" style={{ width: "48%" }} />
          </div>
        </div>
      </div>
      <span className="ask-quote">“Where do I add my domain?”</span>
    </div>
  );
}

function SketchVisual() {
  const { ref, seen } = useInView<SVGSVGElement>();
  const request = sketchArrow([128, 74], [240, 74], -0.08);
  const files = sketchArrow([240, 118], [128, 118], -0.08);
  const live = sketchArrow([110, 150], [110, 196], 0.1);
  return (
    <svg
      ref={ref}
      className={`sketch-visual ${seen ? "ink-draw" : "ink-wait"}`}
      viewBox="0 0 360 250"
      aria-hidden="true"
    >
      <path pathLength={1} d="M22 52h104v96H22Z" />
      <path pathLength={1} className="ink-fill" d="M32 62h84v58H32Z" />
      <path pathLength={1} className="ink-delay" d="M246 52h92v96h-92Z M256 70h72v20h-72Z M256 104h72v20h-72Z" />
      <path pathLength={1} className="ink-delay" d={request.line + request.head} />
      <path pathLength={1} className="ink-delay ink-soft" d={files.line + files.head} />
      <path pathLength={1} className="ink-delay-2" d={live.line + live.head} />
      <path pathLength={1} className="ink-delay-2 ink-green" d="M22 202h226v34H22Z" />
      <path pathLength={1} className="ink-delay-2 ink-red" d={sketchEllipse(300, 214, 30, 24, 7)} />
      <text x="24" y="176">browser</text>
      <text x="262" y="172">hosting</text>
      <text x="160" y="62">request</text>
      <text x="168" y="140">files</text>
      <text x="36" y="224">your site, live</text>
      <text x="295" y="220" className="ink-text-red">?</text>
    </svg>
  );
}

function WalkVisual() {
  const items = [
    ["Open Terminal", "⌘ Space"],
    ["Install the CLI", "brew install"],
    ["Sign in", "astrid login"],
    ["Run your first task", ""],
  ];
  return (
    <ol className="walk-visual">
      {items.map(([label, hint], index) => (
        <li key={label} data-state={index < 1 ? "done" : index === 1 ? "now" : "next"}>
          <span className="walk-dot" />
          <span>{label}</span>
          {hint && <kbd>{hint}</kbd>}
        </li>
      ))}
      <span className="walk-progress">
        STEP 2 OF 4 · <b>WAITING FOR YOU</b>
      </span>
    </ol>
  );
}

function PrivateVisual() {
  return (
    <div className="private-visual">
      {[
        ["Looks only when you ask", "EACH LOOK"],
        ["Frames stay in memory", "10 S"],
        ["Clicks need your yes", "PER ACTION"],
        ["Nothing replayed", "NEVER"],
      ].map(([label, value]) => (
        <div key={label}>
          <i />
          <span>{label}</span>
          <b>{value}</b>
        </div>
      ))}
    </div>
  );
}

function PhoneVisual() {
  return (
    <div className="phone-visual">
      <PhoneMockupCard>
        <div className="phone-app">
          <span className="phone-time">9:41</span>
          <span className="phone-title">Wi‑Fi</span>
          <span className="phone-bubble">
            <b>ASTRID</b> Tap “Other network”, then type the name on the router.
          </span>
          {["Home‑5G", "Office", "Other network…"].map((name, index) => (
            <span key={name} className={`phone-row ${index === 2 ? "target" : ""}`}>
              {name}
            </span>
          ))}
        </div>
      </PhoneMockupCard>
      <div className="phone-orbit" aria-hidden="true">
        <span>LAPTOP</span>
        <i />
        <span>PHONE</span>
      </div>
    </div>
  );
}

const prompts = [
  "why does this test keep failing?",
  "walk me through setting up SSH",
  "draw how a load balancer works",
  "what is this chart actually saying?",
];

function AskVisual() {
  const [index, setIndex] = useState(0);
  useEffect(() => {
    const timer = setInterval(() => setIndex((value) => (value + 1) % prompts.length), 3200);
    return () => clearInterval(timer);
  }, []);
  return (
    <div className="ask-visual">
      <div className="keys" aria-hidden="true">
        <kbd>⌃</kbd>
        <span>+</span>
        <kbd>⌥</kbd>
        <span className="keys-hint">HOLD TO TALK</span>
      </div>
      <div className="ask-bar">
        <span className="ask-orb" aria-hidden="true">
          <i className="orb-swirl" />
          <AstridLogo animated />
        </span>
        <span className="ask-text" key={index}>
          {prompts[index]}
        </span>
        <span className="ask-wave" aria-hidden="true">
          {Array.from({ length: 9 }, (_, bar) => (
            <b key={bar} style={{ animationDelay: `${bar * 90}ms` }} />
          ))}
        </span>
      </div>
      <div className="ask-chips">
        {["Explain this", "Walk me through it", "Draw it", "Do it for me"].map((chip) => (
          <span key={chip}>{chip}</span>
        ))}
      </div>
    </div>
  );
}

export function Bento() {
  return (
    <div className="bento">
      <Tile
        className="tile-point"
        index="01 / POINTS"
        title="It points at the next click."
        visual={<PointVisual />}
      >
        Ask where something is and Astrid’s own cursor glides there, on your
        real screen, in the app you already have open.
      </Tile>
      <Tile
        className="tile-sketch"
        index="02 / SKETCHES"
        title="It draws on top of your work."
        visual={<SketchVisual />}
      >
        Quick ink over anything: arrows, circles and boxes that explain how the
        pieces connect.
      </Tile>
      <Tile
        className="tile-walk"
        index="03 / WALKS YOU THROUGH"
        title="One step at a time."
        visual={<WalkVisual />}
      >
        Astrid waits for you, checks the screen, then moves on. No wall of
        instructions.
      </Tile>
      <Tile
        className="tile-figure"
        index="04 / BUILDS FIGURES"
        title="Diagrams that actually work."
        visual={<EngineFigure theme="light" />}
      >
        Anatomy-built isometric figures with live parts, made for exactly the
        thing you’re trying to understand.
      </Tile>
      <Tile
        className="tile-private"
        index="05 / STAYS YOURS"
        title="It sees only when asked."
        visual={<PrivateVisual />}
      >
        Every look and every action is a separate, visible grant. Screens are
        never stored or replayed.
      </Tile>
      <Tile
        className="tile-phone"
        index="06 / FOLLOWS YOU"
        title="Laptop now. Phone next."
        status="NEXT"
        visual={<PhoneVisual />}
      >
        The same Astrid, on the device in your hand: settings, forms, the router
        behind the TV.
      </Tile>
      <Tile
        className="tile-ask"
        index="07 / JUST ASK"
        title="Hold two keys. Say it out loud."
        visual={<AskVisual />}
      >
        Voice first, typing when you’d rather. Astrid answers where you’re
        looking, not in another chat tab.
      </Tile>
    </div>
  );
}
