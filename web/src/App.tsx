import { useEffect, useRef, useState } from "react";
import { RuneIcon } from "runeicons-react";
import { AstridLogo } from "./components/AstridLogo";
import { LaptopMockupCard } from "./components/LaptopMockup";

const repo = "https://github.com/404khai/astrid";
const doc = (path: string) => `${repo}/blob/main/${path}`;
function Icon({ name = "arrow-right" }: { name?: string }) {
  return (
    <span className="icon" aria-hidden="true">
      <RuneIcon name={`arrows-${name}`} size={20} />
    </span>
  );
}
function Action({
  children,
  href,
  secondary = false,
}: {
  children: React.ReactNode;
  href: string;
  secondary?: boolean;
}) {
  return (
    <a className={`action ${secondary ? "secondary" : ""}`} href={href}>
      {children}
      <Icon name={secondary ? "arrow-right" : "arrow-up-right"} />
    </a>
  );
}
function Navigation() {
  const [open, setOpen] = useState(false);
  const menu = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (!menu.current?.contains(event.target as Node)) setOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        trigger.current?.focus();
      }
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [open]);
  return (
    <header className="navbar">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <nav className="nav-box nav-left" aria-label="Sections">
        <a href="#loop">The loop</a>
        <a href="#philosophy">Philosophy</a>
        <a href="#roadmap">Roadmap</a>
      </nav>
      <a className="brand" href="#" aria-label="Astrid home">
        <AstridLogo animated />
      </a>
      <nav className="nav-box nav-right" aria-label="Project">
        <a href={doc("docs/architecture.md")}>Docs</a>
        <a href={repo}>GitHub</a>
        <div className="more" ref={menu}>
          <button
            ref={trigger}
            onClick={() => setOpen(!open)}
            aria-expanded={open}
            aria-controls="more-links"
          >
            More
            <Icon name="chevron-down" />
          </button>
          {open && (
            <div className="dropdown" id="more-links">
              <a
                href={doc("docs/development.md")}
                onClick={() => setOpen(false)}
              >
                Build notes
                <Icon />
              </a>
              <a href={doc("AGENTS.md")} onClick={() => setOpen(false)}>
                Contributing
                <Icon />
              </a>
              <a href={`${repo}/issues`} onClick={() => setOpen(false)}>
                Issues
                <Icon />
              </a>
            </div>
          )}
        </div>
      </nav>
    </header>
  );
}
function LoopVisual() {
  return (
    <div className="loop-visual" id="loop">
      <svg
        className="circuit"
        viewBox="0 0 1280 440"
        fill="none"
        aria-hidden="true"
      >
        <path
          d="M32 302H190L260 372H1008L1078 302H1248 M32 132H190L260 62H1008L1078 132H1248"
          stroke="#1A32EC"
          strokeWidth="2"
        />
        <path
          d="M42 322H180L250 392H1018L1088 322H1238 M42 112H180L250 42H1018L1088 112H1238"
          stroke="#151B55"
        />
        <rect x="22" y="292" width="20" height="20" fill="#4459F9" />
        <rect x="1238" y="122" width="20" height="20" fill="#4459F9" />
      </svg>
      <aside className="tool-note inspect">
        <span className="eyebrow">01 / INSPECT</span>
        <span>read_file</span>
        <small>src/parser.rs</small>
        <div className="code-bars" aria-hidden="true">
          <i />
          <i />
          <i />
        </div>
      </aside>
      <LaptopMockupCard variant="titanium">
        <div className="terminal">
          <div className="terminal-heading">
            <span>astrid — ~/hello-world</span>
            <span>ILLUSTRATIVE RUN</span>
          </div>
          <div className="terminal-body">
            <p className="prompt">
              <span>❯</span> find and fix the failing parser test
            </p>
            <p className="response">
              I’ll inspect the parser, make a targeted edit,
              <br />
              then run the tests.
            </p>
            <div className="tool-rows">
              {[
                ["01", "read_file", "src/parser.rs", "complete"],
                ["02", "edit_file", "handle empty input", "complete"],
                ["03", "shell", "cargo test", "passed"],
              ].map((row) => (
                <div className="tool-row" key={row[0]}>
                  <span>{row[0]}</span>
                  <span>{row[1]}</span>
                  <span>{row[2]}</span>
                  <span>{row[3]}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </LaptopMockupCard>
      <aside className="tool-note execute">
        <span className="eyebrow">02 / EXECUTE</span>
        <span>cargo test</span>
        <small>test result: ok</small>
        <span className="back-to-model">
          <i /> back to model <Icon />
        </span>
      </aside>
      <p className="loop-caption">
        REQUEST <span>→</span> MODEL <span>→</span> TOOL <span>→</span> RESULT{" "}
        <span>→</span> REPEAT
      </p>
    </div>
  );
}
export function App() {
  return (
    <>
      <Navigation />
      <main id="main">
        <section className="hero" aria-labelledby="hero-title">
          <p className="eyebrow hero-label">
            <i /> ASTRID / INFERENCE-AWARE PERSONAL AGENT RUNTIME
          </p>
          <h1 id="hero-title">
            Inside the loop.
            <br />
            Beyond the black box.
          </h1>
          <p className="hero-description">
            A personal agent runtime for exploring how agents reason,
            <br className="desktop-break" /> use tools, and work with context.
            Built to be understood.
          </p>
          <div className="hero-actions">
            <Action href={repo}>Explore Astrid</Action>
            <Action secondary href={doc("docs/architecture.md")}>
              Read the architecture
            </Action>
          </div>
          <LoopVisual />
        </section>
        <section
          className="section philosophy"
          id="philosophy"
          aria-labelledby="philosophy-title"
        >
          <div className="philosophy-heading">
            <p className="eyebrow">01 / THE PHILOSOPHY</p>
            <div>
              <h2 id="philosophy-title">
                Capable agents deserve
                <br />
                understandable systems.
              </h2>
              <p className="section-description">
                Astrid explores the machinery underneath agent behavior.
                <br className="desktop-break" /> A small, deliberate system for
                learning what makes an agent
                <br className="desktop-break" /> reliable—and what happens when
                it isn’t.
              </p>
            </div>
          </div>
          <div className="principles">
            {[
              [
                "01",
                "Make state explicit.",
                "Runs, turns, tools, and context should have clear boundaries. If it matters, it should be visible.",
              ],
              [
                "02",
                "Measure before optimizing.",
                "Understand where time, tokens, and cost go. Make better decisions with evidence, not guesswork.",
              ],
              [
                "03",
                "Earn the autonomy.",
                "Start with a reliable loop. Delegate within explicit permissions. Add capabilities when the foundations can support them.",
              ],
            ].map(([number, title, description]) => (
              <article key={number}>
                <span className="principle-number">{number}</span>
                <h3>{title}</h3>
                <p>{description}</p>
              </article>
            ))}
          </div>
        </section>
        <section
          className="section roadmap"
          id="roadmap"
          aria-labelledby="roadmap-title"
        >
          <div className="roadmap-heading">
            <div>
              <p className="eyebrow">02 / BUILT IN PHASES</p>
              <h2 id="roadmap-title">
                Start small.
                <br />
                Understand deeply.
              </h2>
            </div>
            <p>
              Each phase asks a better question.
              <br />
              The goal is learning how capable agents work,
              <br className="desktop-break" /> one observable primitive at a
              time.
            </p>
          </div>
          <div className="phase-panel">
            <article className="featured-phase">
              <div className="phase-meta">
                <span>PHASE 04</span>
                <span>FOUNDATION VERIFIED</span>
              </div>
              <h3>See the loop.</h3>
              <p>
                Persistent traces. Context accounting. Inference telemetry.
                <br className="desktop-break" /> Understand where time, tokens,
                and tools go.
                <br className="desktop-break" /> The foundations for bounded
                personal delegation.
              </p>
              <span className="phase-sequence">
                RUN → TRACE → INSPECT → UNDERSTAND
              </span>
            </article>
            <div className="phase-list">
              {[
                ["01", "Runtime model", "BUILT"],
                ["02", "Execution environment", "VERIFIED"],
                ["03", "Context engine", "VERIFIED"],
                ["NEXT", "Mac desktop client", "PLANNED"],
              ].map(([number, title, status]) => (
                <div key={number}>
                  <span>{number}</span>
                  <h3>{title}</h3>
                  <span>{status}</span>
                </div>
              ))}
            </div>
          </div>
          <div className="roadmap-bottom">
            <span>LEARNING → CORRECTNESS → OBSERVABILITY → CLARITY</span>
            <a href={doc("docs/plans/personal-agent-roadmap.md")}>
              Explore the full roadmap
              <Icon />
            </a>
          </div>
        </section>
      </main>
      <footer>
        <div className="footer-content">
          <div className="footer-callout">
            <div>
              <p className="eyebrow">A WORK IN PROGRESS. BY DESIGN.</p>
              <h2>Build it. Understand it.</h2>
            </div>
            <Action href={repo}>Explore Astrid</Action>
          </div>
          <div className="footer-links">
            <div className="footer-brand">
              <AstridLogo />
              <p>
                A personal agent runtime.
                <br />
                Built to learn. Built to be understood.
              </p>
            </div>
            {[
              {
                title: "EXPLORE",
                links: [
                  ["The loop", "#loop"],
                  ["Philosophy", "#philosophy"],
                  ["Roadmap", "#roadmap"],
                ],
              },
              {
                title: "PROJECT",
                links: [
                  ["GitHub", repo],
                  ["Docs", doc("docs/architecture.md")],
                  ["AGENTS.md", doc("AGENTS.md")],
                ],
              },
              {
                title: "PARTICIPATE",
                links: [
                  ["Issues", `${repo}/issues`],
                  ["Contributing", doc("AGENTS.md")],
                  ["Build notes", doc("docs/development.md")],
                ],
              },
            ].map(({ title, links }) => (
              <nav key={title} aria-label={title}>
                <span className="eyebrow">{title}</span>
                {links.map(([label, href]) => (
                  <a key={label} href={href}>
                    {label}
                  </a>
                ))}
              </nav>
            ))}
          </div>
          <div className="footer-meta">
            <span>© 2026 ASTRID</span>
            <span>AN EXPERIMENT IN UNDERSTANDING AGENTS.</span>
            <span>DESIGNED & BUILT BY 404KHAI</span>
          </div>
        </div>
        <div className="wordmark" aria-hidden="true">
          ASTRID
        </div>
      </footer>
    </>
  );
}
