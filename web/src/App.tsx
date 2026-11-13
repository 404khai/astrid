import { useEffect, useRef, useState } from "react";
import { RuneIcon } from "runeicons-react";
import { AstridLogo } from "./components/AstridLogo";
import { AstridHome } from "./components/AstridHome";
import { BrandLink } from "./components/BrandMenu";
import { Bento } from "./components/Bento";
import { DnaExplainer } from "./components/DnaExplainer";
import { Everyday } from "./components/Everyday";
import { HeroScreen } from "./components/HeroScreen";
import { LaptopMockupCard } from "./components/LaptopMockup";
import { SpatialStack } from "./components/SpatialStack";
import { VerseText } from "./components/VerseText";

/** Section headings print in VerseText; "\n" keeps the designed line breaks. */
function SectionTitle({ id, text }: { id?: string; text: string }) {
  return (
    <div className="h2" role="heading" aria-level={2} id={id}>
      <VerseText text={text} tone="dark" idle={false} seed={text.length * 31} />
    </div>
  );
}

const repo = "https://github.com/404khai/astrid";
const doc = (path: string) => `${repo}/blob/main/${path}`;
const gameplan = `${repo}/blob/v2/docs/plans/v2-gameplan.md`;
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
  const [mobileOpen, setMobileOpen] = useState(false);
  const mobileMenu = useRef<HTMLDivElement>(null);
  const mobileTrigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open && !mobileOpen) return;
    const outside = (event: PointerEvent) => {
      if (!menu.current?.contains(event.target as Node)) setOpen(false);
      if (!mobileMenu.current?.contains(event.target as Node))
        setMobileOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        if (mobileOpen) {
          setMobileOpen(false);
          mobileTrigger.current?.focus();
        } else trigger.current?.focus();
      }
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [open, mobileOpen]);
  return (
    <header className="navbar">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <div className="mobile-menu" ref={mobileMenu}>
        <button
          className="mobile-nav-box"
          ref={mobileTrigger}
          aria-expanded={mobileOpen}
          aria-controls="mobile-links"
          onClick={() => setMobileOpen(!mobileOpen)}
        >
          Menu
          <Icon name="chevron-down" />
        </button>
        {mobileOpen && (
          <nav
            className="mobile-dropdown"
            id="mobile-links"
            aria-label="Mobile navigation"
          >
            {[
              ["See it work", "#demo"],
              ["How it helps", "#help"],
              ["Spatial", "#spatial"],
              ["Gameplan", "#gameplan"],
              ["Docs", "/docs.html"],
              ["Build notes", doc("docs/development.md")],
              ["Contributing", doc("AGENTS.md")],
              ["Issues", `${repo}/issues`],
            ].map(([label, href]) => (
              <a key={label} href={href} onClick={() => setMobileOpen(false)}>
                {label}
                <Icon />
              </a>
            ))}
          </nav>
        )}
      </div>
      <nav className="nav-box nav-left" aria-label="Sections">
        <a href="#help">How it helps</a>
        <a href="#spatial">Spatial</a>
        <a href="#gameplan">Gameplan</a>
      </nav>
      <BrandLink className="brand" href="#" label="Astrid home">
        <AstridLogo animated tracking />
      </BrandLink>
      <a className="mobile-nav-box mobile-github" href={repo}>
        GitHub
        <Icon name="arrow-up-right" />
      </a>
      <nav className="nav-box nav-right" aria-label="Project">
        <a href="/docs.html">Docs</a>
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
function DemoVisual() {
  return (
    <div className="loop-visual demo-visual" id="demo">
      <svg
        className="circuit"
        viewBox="0 0 1280 640"
        fill="none"
        aria-hidden="true"
      >
        <path
          d="M32 452H170L240 522H1040L1110 452H1248 M32 182H170L240 112H1040L1110 182H1248"
          stroke="#1A32EC"
          strokeWidth="2"
        />
        <path
          d="M42 472H160L230 542H1050L1120 472H1238 M42 162H160L230 92H1050L1120 162H1238"
          stroke="#151B55"
        />
        <rect x="22" y="442" width="20" height="20" fill="#4459F9" />
        <rect x="1238" y="172" width="20" height="20" fill="#4459F9" />
      </svg>
      <p className="demo-note demo-note-left">
        <span className="eyebrow">01 / LOOK</span>
        <strong>You ask. It looks once.</strong>
        <small>One frame of Preview, seen only because you asked.</small>
      </p>
      <LaptopMockupCard variant="titanium">
        <HeroScreen />
      </LaptopMockupCard>
      <p className="demo-note demo-note-right">
        <span className="eyebrow">02 / EXPLAIN</span>
        <strong>Then it shows you.</strong>
        <small>An engine built from page 4, walked through part by part.</small>
      </p>
      <p className="loop-caption">
        LOOK <span>→</span> POINT <span>→</span> SKETCH <span>→</span> EXPLAIN{" "}
        <span>→</span> CHECK
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
            <i /> ASTRID / A PERSONAL AGENT THAT SEES WHAT YOU SEE
          </p>
          <h1 id="hero-title">
            Don’t just get answers.
            <br />
            Get <span className="sweep">shown.</span>
          </h1>
          <p className="hero-description">
            Astrid looks at your screen when you ask, then points, sketches and
            <br className="desktop-break" /> builds working diagrams right
            where you’re stuck. Laptop now, phone next.
          </p>
          <div className="hero-actions">
            <Action href={repo}>Explore Astrid</Action>
            <Action secondary href={gameplan}>
              Read the gameplan
            </Action>
          </div>
          <DemoVisual />
        </section>
        <section
          className="section help"
          id="help"
          aria-labelledby="help-title"
        >
          <div className="philosophy-heading">
            <p className="eyebrow">01 / HOW IT HELPS</p>
            <div>
              <SectionTitle id="help-title" text={"Less chat window.\nMore over-the-shoulder."} />
              <p className="section-description">
                The best help is someone beside you who can see the screen,
                <br className="desktop-break" /> point at the thing and draw
                you a picture. Astrid is built to be that.
              </p>
            </div>
          </div>
          <Bento />
        </section>
        <section
          className="section everyday-section"
          id="everyday"
          aria-labelledby="everyday-title"
        >
          <div className="philosophy-heading">
            <p className="eyebrow">02 / EVERYDAY</p>
            <div>
              <SectionTitle id="everyday-title" text={"The small stuff,\nhandled out loud."} />
              <p className="section-description">
                Timers, alarms, your calendar, that one email from last March.
                <br className="desktop-break" /> The basics a personal agent
                should just do, planned for v2.
              </p>
            </div>
          </div>
          <div className="band-stage">
            <Everyday />
          </div>
        </section>
        <section className="explainer-section" id="explain" aria-label="Explanations with a pointer">
          <DnaExplainer />
        </section>
        <section
          className="section spatial-section"
          id="spatial"
          aria-labelledby="spatial-title"
        >
          <div className="roadmap-heading">
            <div>
              <p className="eyebrow">03 / SPATIAL BY DESIGN</p>
              <SectionTitle id="spatial-title" text={"A layer over\neverything you use."} />
            </div>
            <p>
              Astrid doesn’t replace your apps or pull you into its own.
              <br className="desktop-break" /> It floats above them in three
              thin, honest layers.
            </p>
          </div>
          <SpatialStack />
        </section>
        <section
          className="section home-section"
          id="home"
          aria-labelledby="home-title"
        >
          <div className="philosophy-heading">
            <p className="eyebrow">04 / HOME BASE</p>
            <div>
              <SectionTitle id="home-title" text={"One quiet place\nfor everything it did."} />
              <p className="section-description">
                Every ask, sketch and walkthrough lands back here, with the
                <br className="desktop-break" /> devices it can see and what it
                is allowed to do in plain view.
              </p>
            </div>
          </div>
          <AstridHome />
        </section>
        <section
          className="section philosophy"
          id="philosophy"
          aria-labelledby="philosophy-title"
        >
          <div className="philosophy-heading">
            <p className="eyebrow">05 / UNDERNEATH</p>
            <div>
              <SectionTitle id="philosophy-title" text={"Personal means\nanswerable to you."} />
              <p className="section-description">
                Under the polish is Astrid’s own Rust runtime: explicit
                grants,
                <br className="desktop-break" /> finite limits and traces you
                can read. It earns every look.
              </p>
            </div>
          </div>
          <div className="principles">
            {[
              [
                "01",
                "Asks before it looks.",
                "Each screenshot is its own single-use grant. No ambient watching, no silent recording, no stored frames.",
              ],
              [
                "02",
                "Shows its work.",
                "What it saw, why it pointed there and which model answered are all on the record, in metadata-only traces.",
              ],
              [
                "03",
                "Earns the autonomy.",
                "Explaining comes first. Clicking for you comes later, one approved action at a time, on a fresh frame.",
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
          id="gameplan"
          aria-labelledby="roadmap-title"
        >
          <div className="roadmap-heading">
            <div>
              <p className="eyebrow">06 / THE GAMEPLAN</p>
              <SectionTitle id="roadmap-title" text={"Explain first.\nThen do."} />
            </div>
            <p>
              Each milestone ships something you can feel on your own screen,
              <br className="desktop-break" /> on top of the runtime that is
              already built.
            </p>
          </div>
          <div className="phase-panel">
            <article className="featured-phase">
              <div className="phase-meta">
                <span>V2 · 01</span>
                <span>IN PROGRESS</span>
              </div>
              <h3>Astrid on your Mac.</h3>
              <p>
                A floating presence over every app: its own cursor,
                <br className="desktop-break" /> push-to-talk voice and a
                sketch layer.
                <br className="desktop-break" /> Sees one frame when you ask.
              </p>
              <span className="phase-sequence">
                ASK → LOOK → POINT → SKETCH → EXPLAIN
              </span>
            </article>
            <div className="phase-list">
              {[
                ["02", "Anatomy figures, live", "NEXT"],
                ["03", "Guided walkthroughs", "PLANNED"],
                ["04", "Phone companion", "PLANNED"],
                ["05", "Memory you control", "LATER"],
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
            <span>SEE → EXPLAIN → GUIDE → ACT</span>
            <a href={gameplan}>
              Read the full gameplan
              <Icon />
            </a>
          </div>
        </section>
      </main>
      <footer>
        <div className="footer-content">
          <div className="footer-callout">
            <div>
              <p className="eyebrow">A PERSONAL AGENT. BUILT IN THE OPEN.</p>
              <SectionTitle text={"See it. Understand it."} />
            </div>
            <Action href={repo}>Explore Astrid</Action>
          </div>
          <div className="footer-links">
            <div className="footer-brand">
              <AstridLogo />
              <p>
                A personal agent that sees what you see.
                <br />
                Points, sketches and explains.
              </p>
            </div>
            {[
              {
                title: "EXPLORE",
                links: [
                  ["See it work", "#demo"],
                  ["How it helps", "#help"],
                  ["Spatial", "#spatial"],
                ],
              },
              {
                title: "PROJECT",
                links: [
                  ["GitHub", repo],
                  ["Gameplan", gameplan],
                  ["Docs", "/docs.html"],
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
            <span>HELP THAT LIVES ON YOUR SCREEN.</span>
            <span>DESIGNED & BUILT BY 404KHAI</span>
          </div>
        </div>
        <div className="wordmark" aria-hidden="true">
          <VerseText text="ASTRID" tone="dark" seed={2026} />
        </div>
      </footer>
    </>
  );
}
