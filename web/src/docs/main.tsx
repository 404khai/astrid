import React, { useEffect, useState, type ReactNode } from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/space-grotesk/latin-400.css";
import "@fontsource/space-grotesk/latin-500.css";
import "@fontsource/space-grotesk/latin-700.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import { AstridCursor } from "../components/AstridCursor";
import { AstridLogo } from "../components/AstridLogo";
import { BrandLink } from "../components/BrandMenu";
import { BounceSidebar } from "../components/nexvyn/BounceSidebar";
import { NavigationCompass } from "../components/nexvyn/NavigationCompass";
import { ScrollIndicator } from "../components/nexvyn/ScrollIndicator";
import { VerseText } from "../components/VerseText";
import { CursorGrid } from "./Cursors";
import { ListeningCard, OrbGrid, OutputBox } from "./AgentStates";
import { AskBox, ComputerUseGrid } from "./ComputerUse";
import "../styles.css";
import "./cursors.css";
import "./agent-states.css";
import "./computer-use.css";
import "./docs.css";

const repo = "https://github.com/404khai/astrid";
const pages = [
  { id: "overview", title: "Overview" },
  { id: "brand", title: "Brand guidelines" },
  { id: "components", title: "Components" },
] as const;
type PageId = (typeof pages)[number]["id"];

/** A page section: registers with the scroll indicator by index. */
function Section({ index, title, eyebrow, children }: { index: number; title: string; eyebrow: string; children: ReactNode }) {
  return (
    <section className="doc-section" data-section-index={index} id={title.toLowerCase().replace(/\W+/g, "-")}>
      <p className="eyebrow">{eyebrow}</p>
      <div className="h2 doc-h2" role="heading" aria-level={2}>
        <VerseText text={title} tone="dark" idle={false} seed={title.length * 13} />
      </div>
      {children}
    </section>
  );
}

function Overview() {
  return (
    <>
      <section className="doc-hero" data-section-index={0}>
        <div>
          <p className="eyebrow">ASTRID / DOCS</p>
          <h1>Find your bearings.</h1>
          <p className="doc-lede">
            Everything that makes Astrid look and feel like Astrid: the mark, the blues, the
            cursor and the pieces the site is built from. Drag the dial, or just scroll.
          </p>
        </div>
        <NavigationCompass
          links={[
            { angle: 345, label: "Brand", href: "#brand" },
            { angle: 75, label: "Components", href: "#components" },
            { angle: 165, label: "Gameplan", href: `${repo}/blob/v2/docs/plans/v2-gameplan.md` },
            { angle: 255, label: "Home", href: "/" },
          ]}
        />
      </section>
      <Section index={1} eyebrow="01 / WHAT'S HERE" title="Three places to start">
        <div className="doc-cards">
          {[
            ["Brand guidelines", "The mark, colour, type and the multi-blue backgrounds.", "#brand"],
            ["Components", "Astrid with all twenty cursors, plus the site's building blocks.", "#components"],
            ["Gameplan", "Where Astrid is going and the decisions still open.", `${repo}/blob/v2/docs/plans/v2-gameplan.md`],
          ].map(([title, body, href]) => (
            <a key={title} className="doc-card" href={href}>
              <b>{title}</b>
              <span>{body}</span>
            </a>
          ))}
        </div>
      </Section>
    </>
  );
}

const bands = (from: string[]) => `linear-gradient(90deg, ${from.map((c, i) => `${c} ${i * 10}% ${(i + 1) * 10}%`).join(", ")})`;
const LIGHT = ["#1a32ec", "#2a40f0", "#3b51f4", "#4c61f6", "#5d71f8", "#6e81fa", "#8090fb", "#93a1fc", "#a7b2fd", "#bcc4fe"];
const DARK = ["#060a2a", "#0a1040", "#0e1656", "#121c6c", "#162282", "#1a2898", "#1d2eae", "#1a32ec", "#2a40f0", "#3b51f4"];

function Brand() {
  return (
    <>
      <Section index={0} eyebrow="01 / MARK" title="The mark">
        <p className="doc-text">
          Two arches and a pair of eyes. The eyes always track the pointer; when Astrid is
          working they glance and blink. Never put the mark on a plate or a black tile: it sits
          straight on the surface.
        </p>
        <div className="brand-marks">
          <div className="on-dark"><AstridLogo tracking /></div>
          <div className="on-light"><AstridLogo tracking /></div>
          <div className="on-bands" style={{ background: bands(LIGHT) }}><AstridLogo tracking /></div>
          <div className="on-cursor"><AstridCursor x={90} y={70} /></div>
        </div>
      </Section>
      <Section index={1} eyebrow="02 / COLOUR" title="Colour">
        <div className="swatches">
          {[
            ["Ground", "#121212"],
            ["Astrid blue", "#4459F9"],
            ["Action blue", "#1A32EC"],
            ["Ink blue", "#8793FF"],
            ["Mist", "#B8C0FF"],
            ["Eyes", "#00F7D5"],
            ["Paper", "#EEEEEE"],
            ["Line", "#292929"],
          ].map(([name, hex]) => (
            <div key={hex} className="swatch">
              <i style={{ background: hex }} />
              <b>{name}</b>
              <code>{hex}</code>
            </div>
          ))}
        </div>
        <div className="ramp">
          {LIGHT.concat(DARK.slice().reverse()).map((c, i) => (
            <i key={i} style={{ background: c }} title={c} />
          ))}
        </div>
      </Section>
      <Section index={2} eyebrow="03 / BACKGROUNDS" title="Multi-blue backgrounds">
        <p className="doc-text">
          Ten-step blue bands are the brand's signature surface. Light bands over dark bands,
          poster blocks around a paper panel, or a single ramp. Content always sits on its own
          card or panel above them.
        </p>
        <div className="bg-grid">
          <figure>
            <div className="bg bg-split" style={{ background: `${bands(LIGHT)} top / 100% 50% no-repeat, ${bands(DARK)} bottom / 100% 50% no-repeat` }}>
              <span className="bg-card">Split bands</span>
            </div>
            <figcaption>Split bands: Everyday and Home base</figcaption>
          </figure>
          <figure>
            <div className="bg bg-poster">
              <i className="pp a" /><i className="pp b" /><i className="pp c" /><i className="pp d" />
              <span className="bg-paper">Poster</span>
              <span className="bg-coord">S02 · C1 · 04.4,03.1</span>
            </div>
            <figcaption>Poster blocks: the whiteboard section</figcaption>
          </figure>
          <figure>
            <div className="bg" style={{ background: bands(LIGHT) }}>
              <span className="bg-card">Light ramp</span>
            </div>
            <figcaption>Light ramp</figcaption>
          </figure>
          <figure>
            <div className="bg" style={{ background: bands(DARK) }}>
              <span className="bg-card dark">Night ramp</span>
            </div>
            <figcaption>Night ramp</figcaption>
          </figure>
          <figure>
            <div className="bg" style={{ background: `${bands(LIGHT.slice().reverse())}` }}>
              <span className="bg-card">Reversed</span>
            </div>
            <figcaption>Reversed ramp</figcaption>
          </figure>
          <figure>
            <div className="bg bg-vertical" style={{ background: `linear-gradient(180deg, ${LIGHT.map((c, i) => `${c} ${i * 10}% ${(i + 1) * 10}%`).join(", ")})` }}>
              <span className="bg-card">Vertical</span>
            </div>
            <figcaption>Vertical bands</figcaption>
          </figure>
        </div>
      </Section>
      <Section index={3} eyebrow="04 / TYPE" title="Type">
        <div className="type-specimens">
          <div><span className="eyebrow">DISPLAY · SPACE GROTESK 500</span><p className="ts-display">Don’t just get answers.</p></div>
          <div><span className="eyebrow">TEXT · SPACE GROTESK 400</span><p className="ts-text">Astrid looks at your screen when you ask, then points, sketches and builds working diagrams.</p></div>
          <div><span className="eyebrow">LABELS · IBM PLEX MONO</span><p className="ts-mono">01 / HOW IT HELPS · LOOK → POINT → SKETCH</p></div>
          <div><span className="eyebrow">HEADINGS PRINT IN VERSETEXT</span><VerseText text="See it. Understand it." tone="dark" seed={5} className="ts-verse" /></div>
        </div>
      </Section>
    </>
  );
}

function Components() {
  return (
    <>
      <Section index={0} eyebrow="01 / CURSORS" title="Astrid, with every cursor">
        <p className="doc-text">
          The twenty cursors from <code>references/cursors.jpg</code>, each with the mark trailing its
          hotspot. Hover a card and it follows your mouse. <b>Arrow click</b> (17) is Astrid’s own
          cursor everywhere on the site.
        </p>
        <CursorGrid />
      </Section>
      <Section index={1} eyebrow="02 / AGENT STATES" title="What Astrid is doing">
        <p className="doc-text">
          The mark carries a thinking orb beside it, the way it trails each cursor, so you can
          always tell whether Astrid is looking, solving or writing. Orbs from{" "}
          <code>thinking-orbs</code>, listening from <code>voice-ring</code>, and the answer
          surface from <code>liquid-gooey</code>.
        </p>
        <OrbGrid />
        <div className="as-demo">
          <ListeningCard />
          <OutputBox />
        </div>
      </Section>
      <Section index={2} eyebrow="03 / COMPUTER USE" title="Astrid at work on screen">
        <p className="doc-text">
          When Astrid operates your Mac, its cursor carries the mark and a state orb, and the
          presence step box says what it is doing. Each action has its own cursor and eyes:
          looking, pointing for <b>Show me</b>, then clicking, typing or scrolling only after
          <b> Do it</b>, and stopping at anything it isn’t allowed to do.
        </p>
        <ComputerUseGrid />
        <AskBox />
      </Section>
      <Section index={3} eyebrow="04 / FROM NEXVYN" title="Navigation pieces">
        <p className="doc-text">
          Adapted from Nexvyn UI to Astrid’s scoped CSS, without extra animation libraries: the
          bounce sidebar on the left, the scroll indicator on the right, and the navigation
          compass on the overview. The laptop and phone mockups on the homepage come from the
          same library.
        </p>
      </Section>
    </>
  );
}

const sectionTitles: Record<PageId, string[]> = {
  overview: ["Compass", "Start here"],
  brand: ["Mark", "Colour", "Backgrounds", "Type"],
  components: ["Cursors", "Agent states", "Computer use", "Navigation"],
};

function Docs() {
  const fromHash = (): PageId => {
    const id = location.hash.slice(1);
    return (pages.find((page) => page.id === id)?.id ?? "overview") as PageId;
  };
  const [page, setPage] = useState<PageId>(fromHash);
  useEffect(() => {
    const onHash = () => {
      setPage(fromHash());
      window.scrollTo(0, 0);
    };
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);
  const index = pages.findIndex((entry) => entry.id === page);
  return (
    <div className="docs">
      <aside className="docs-side">
        <BrandLink className="docs-brand" href="/" label="Astrid home">
          <AstridLogo animated tracking />
          <span>ASTRID</span>
          <small>DOCS</small>
        </BrandLink>
        <BounceSidebar
          items={pages.map((entry) => entry.title)}
          value={index}
          onChange={(next) => {
            location.hash = pages[next].id;
          }}
        />
        <nav className="docs-out">
          <a href="/">Homepage ↗</a>
          <a href={repo}>GitHub ↗</a>
        </nav>
      </aside>
      <main className="docs-main" key={page}>
        {page === "overview" && <Overview />}
        {page === "brand" && <Brand />}
        {page === "components" && <Components />}
      </main>
      <aside className="docs-rail" aria-label="On this page">
        <ScrollIndicator key={page} sections={sectionTitles[page].map((title) => ({ id: title, title }))} />
      </aside>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Docs />
  </React.StrictMode>,
);
