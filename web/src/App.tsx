import { useState } from "react";
import { AstridLogo } from "./components/AstridLogo";
import { BrandLink } from "./components/BrandMenu";
import { Bento } from "./components/Bento";
import { DnaExplainer } from "./components/DnaExplainer";
import { Everyday } from "./components/Everyday";
import { SpatialStack } from "./components/SpatialStack";
import { Action, Footer, Navigation, SectionTitle, Icon, doc, gameplan, repo } from "./components/SiteChrome";
import { VerseText } from "./components/VerseText";

/** The Mac app in three real screenshots, stacked like windows; pick one to bring it forward. */
const shots = [
  { src: "/hero/connections.webp", label: "Phone", alt: "Astrid for Mac: a QR code to connect a phone to this Mac" },
  { src: "/hero/spatial-voice.webp", label: "Voice + canvas", alt: "Astrid for Mac in voice mode, drawing an isometric circuit board while it thinks" },
  { src: "/hero/diagrams.webp", label: "Diagrams", alt: "Astrid for Mac: a grid of diagrams it drew, from an HTTP request to the human heart" },
];
function HeroShots() {
  const [front, setFront] = useState(1);
  return (
    <div className="hero-shots">
      <div className="hero-shots-stage">
        {shots.map((shot, index) => {
          const slot = index === front ? "front" : (index - front + shots.length) % shots.length === 1 ? "right" : "left";
          return (
            <button
              key={shot.src}
              type="button"
              className={`hero-shot hero-shot-${slot}`}
              onClick={() => setFront(index)}
              aria-label={`Show ${shot.label}`}
              aria-pressed={index === front}
              tabIndex={index === front ? -1 : 0}
            >
              <img src={shot.src} alt={shot.alt} width={1600} height={1290} loading={index === 1 ? "eager" : "lazy"} />
            </button>
          );
        })}
      </div>
      <div className="hero-shots-tabs" role="group" aria-label="Screenshots">
        {shots.map((shot, index) => (
          <button key={shot.label} type="button" aria-pressed={index === front} onClick={() => setFront(index)}>
            {shot.label}
          </button>
        ))}
      </div>
    </div>
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
        <span className="eyebrow">01 / ASK</span>
        <strong>Talk to it.</strong>
        <small>Voice mode answers out loud and draws as it explains.</small>
      </p>
      <HeroShots />
      <p className="demo-note demo-note-right">
        <span className="eyebrow">02 / KEEP</span>
        <strong>Every drawing stays.</strong>
        <small>Chats and diagrams live in the Mac app, ready to revisit.</small>
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
          <div className="home-stage">
            <figure className="home-shot">
              <img
                src="/hero/chat.webp"
                alt="Astrid for Mac: a new chat, with past chats, diagrams and blobs in the sidebar"
                width={1600}
                height={1287}
                loading="lazy"
              />
            </figure>
          </div>
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
                  ["Docs", "/docs"],
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
