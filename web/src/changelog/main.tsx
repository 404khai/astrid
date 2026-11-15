import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/space-grotesk/latin-400.css";
import "@fontsource/space-grotesk/latin-500.css";
import "@fontsource/space-grotesk/latin-700.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import { ThinkingOrb } from "thinking-orbs";
import type { OrbState } from "thinking-orbs";
import { AstridLogo, type EyeExpression } from "../components/AstridLogo";
import { Footer, Navigation, repo } from "../components/SiteChrome";
import { VerseText } from "../components/VerseText";
import "../styles.css";
import "./changelog.css";

const PR = `${repo}/pull/`;

type Entry = {
  date: string; // ISO
  title: string;
  tag: "Feature" | "Platform" | "Runtime" | "Foundation";
  lede: string;
  items: { title: string; body: string; pr: number }[];
  /** The mark's mood beside the entry: eyes and orb for what shipped. */
  mood?: [EyeExpression, OrbState];
};

// From the merged PRs on main, newest first.
const ENTRIES: Entry[] = [
  {
    date: "2026-11-15",
    title: "Blob groups on the desktop",
    tag: "Feature",
    lede: "The native app learns to group blobs, peek more quietly and close from the edge of the window.",
    items: [
      { title: "Blob groups", body: "Related blobs gather into groups you can open and tidy together.", pr: 50 },
      { title: "A quieter peek", body: "Peeking at a chat no longer pulls focus from what you were doing.", pr: 50 },
      { title: "Close from the edge", body: "Drag the window off its edge to put it away.", pr: 50 },
    ],
    mood: ["weaving", "weaving"],
  },
  {
    date: "2026-11-14",
    title: "Astrid on your phone",
    tag: "Platform",
    lede: "An Android companion pairs with your Mac by QR code, floats over your apps and talks back.",
    items: [
      { title: "Android companion", body: "A floating bubble, voice mode and blobs, with asks running on your Mac.", pr: 48 },
      { title: "Sidebar and voice page", body: "Chats in a sidebar, a dedicated voice page and the voice ring in the bubble.", pr: 49 },
    ],
    mood: ["connecting", "connecting"],
  },
  {
    date: "2026-11-14",
    title: "Operator: Show me, then Do it",
    tag: "Feature",
    lede: "Guided walkthroughs on your screen. Astrid points with the right cursor for each step and acts only when you say so.",
    items: [
      { title: "/operator walkthroughs", body: "Step cards with Show me and Do it, one look per press and bounded, freshness-checked actions.", pr: 45 },
      { title: "Cursor styles", body: "Each step points with its own cursor from the docs: click, type, scroll and more.", pr: 45 },
      { title: "Computer-use components", body: "The cursor, mark, state orb and step box documented as components.", pr: 46 },
      { title: "Diagrams grid and haptics", body: "The desktop app gains a diagrams grid, sidebar haptics and the coloured mark.", pr: 47 },
    ],
    mood: ["working", "working"],
  },
  {
    date: "2026-11-13",
    title: "A native Mac app",
    tag: "Platform",
    lede: "Astrid.app keeps your chats, diagrams and themes in a Liquid Glass window, separate from the presence on screen.",
    items: [
      { title: "Chats and diagrams", body: "A sidebar of chats titled from their first message, with diagrams inline.", pr: 43 },
      { title: "Themes", body: "Themes for the sidebar and the content, with a settings sheet.", pr: 43 },
      { title: "Distortion Waves intro", body: "The first time Astrid appears, a wave runs up the edges of your screen.", pr: 44 },
    ],
    mood: ["idle", "breathing"],
  },
  {
    date: "2026-11-13",
    title: "Talk to it, and watch it draw",
    tag: "Feature",
    lede: "Push to talk, hear the answer, and see it sketched on a live canvas right on your screen.",
    items: [
      { title: "Push-to-talk voice", body: "On-device speech with the voice ring reacting to your voice.", pr: 39 },
      { title: "Spoken replies", body: "Answers read aloud with local Kokoro TTS, and you can interrupt.", pr: 40 },
      { title: "Live canvas", body: "Voice asks become diagrams drawn by trusted templates in a canvas panel.", pr: 42 },
      { title: "Ask from the mark", body: "A gooey input under the on-screen mark, with brief, structured answers.", pr: 38 },
    ],
    mood: ["listening", "listening"],
  },
  {
    date: "2026-11-13",
    title: "Mail, timers and more models",
    tag: "Runtime",
    lede: "The everyday basics arrive, read-only and permission-first, alongside three new model providers.",
    items: [
      { title: "Read-only mail", body: "Apple Mail via Spotlight and Gmail with gmail.readonly. Astrid can’t send or change mail.", pr: 31 },
      { title: "Timers and alarms", body: "Owned by the resident menu-bar host, so they ring even when nothing else is open.", pr: 31 },
      { title: "Claude providers", body: "The Anthropic Messages API for claude-* models, and claude-code/* through the Claude Agent SDK.", pr: 36 },
      { title: "OpenCode Go", body: "opencode/* models over Chat Completions.", pr: 33 },
    ],
    mood: ["composing", "composing"],
  },
  {
    date: "2026-11-13",
    title: "The mark, rounded and expressive",
    tag: "Foundation",
    lede: "A softer mark with eyes that show what Astrid is doing, and a brand menu on right-click.",
    items: [
      { title: "Rounded corners", body: "The new mark everywhere: navbar, footer, docs, favicon and the presence app.", pr: 41 },
      { title: "Agent states", body: "Nine thinking orbs, each with its own eye expression, a voice ring and a liquid answer box.", pr: 37 },
      { title: "Brand menu", body: "Right-click the logo to copy it as SVG or open the brand guidelines.", pr: 34 },
    ],
    mood: ["sketching", "shaping"],
  },
  {
    date: "2026-11-11",
    title: "It sees what you see",
    tag: "Feature",
    lede: "Astrid becomes an over-the-shoulder agent: a presence on your Mac that looks when you ask and points at the answer.",
    items: [
      { title: "Presence overlay", body: "A native macOS host draws Astrid’s mark and cursor above every app.", pr: 29 },
      { title: "Local look", body: "One approved frame of your screen, captured at 1x without the overlay, never stored.", pr: 30 },
      { title: "Foreground computer use", body: "Ephemeral vision plus bounded clicks, typing, scrolling and keys.", pr: 26 },
      { title: "A new website", body: "Astrid redefined as an on-screen personal agent, with docs.", pr: 28 },
    ],
    mood: ["searching", "searching"],
  },
  {
    date: "2026-10-21",
    title: "Remote machines, with exact grants",
    tag: "Runtime",
    lede: "Commands on an InstaBox machine only with a single-use grant for that machine and that command.",
    items: [
      { title: "Exact-target tool", body: "instabox_exec runs on one named machine, consuming its grant before the request.", pr: 22 },
      { title: "CLI approvals", body: "Foreground approvals for remote commands, with denial, timeout and cancellation.", pr: 23 },
      { title: "Admission across tool batches", body: "Runs stop when accumulated tool results would exceed the context budget.", pr: 21 },
      { title: "Live token usage", body: "Provider usage shown as it streams, with honest coverage.", pr: 20 },
    ],
    mood: ["connecting", "connecting"],
  },
  {
    date: "2026-10-16",
    title: "Routing and workers",
    tag: "Runtime",
    lede: "Different models for different work, and read-only tasks that run side by side.",
    items: [
      { title: "Observable model routing", body: "Deterministic routes with the choice and its reason in every trace.", pr: 14 },
      { title: "Context meter", body: "Always-visible context budget, and routing that survives resume.", pr: 15 },
      { title: "Scheduler and workers", body: "Bounded read-only tasks with dependencies, cancellation and reconciliation.", pr: 16 },
    ],
    mood: ["solving", "solving"],
  },
  {
    date: "2026-10-08",
    title: "Context you can see",
    tag: "Runtime",
    lede: "Budgets, compaction, sessions and traces turn the loop into something you can inspect.",
    items: [
      { title: "Context budgeting", body: "Estimated admission, whole-exchange selection and explained compaction.", pr: 5 },
      { title: "Sessions and modes", body: "Conversations that continue, with ask, auto and unbound permission modes.", pr: 6 },
      { title: "Observability", body: "Optional, metadata-only traces of every model and tool call.", pr: 7 },
      { title: "Targeted reads", body: "Line-range reads and paginated search with context.", pr: 8 },
    ],
    mood: ["working", "working"],
  },
  {
    date: "2026-10-06",
    title: "The loop",
    tag: "Foundation",
    lede: "Astrid starts as a small Rust runtime: request, model, tool, result, repeat.",
    items: [
      { title: "Runtime model", body: "Runs, turns, model and tool calls as explicit state, with events and cancellation.", pr: 1 },
      { title: "Terminal UI", body: "A terminal client redesigned around runtime events.", pr: 2 },
      { title: "Execution environment", body: "Timeouts, output limits, workspace boundaries, permissions and diffs.", pr: 3 },
    ],
    mood: ["idle", "breathing"],
  },
];

const fmt = (iso: string, opts: Intl.DateTimeFormatOptions) =>
  new Date(`${iso}T12:00:00Z`).toLocaleDateString("en-US", { ...opts, timeZone: "UTC" });
const monthKey = (iso: string) => iso.slice(0, 7);
const MONTHS = [...new Set(ENTRIES.map((entry) => monthKey(entry.date)))];

function Changelog() {
  const [month, setMonth] = useState(MONTHS[0]);
  useEffect(() => {
    const onScroll = () => {
      let current = MONTHS[0];
      for (const el of document.querySelectorAll<HTMLElement>("[data-month]")) {
        if (el.getBoundingClientRect().top < window.innerHeight * 0.3) current = el.dataset.month!;
      }
      setMonth(current);
    };
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);
  return (
    <>
      <Navigation />
      <main id="main" className="changelog">
        <header className="cl-head">
          <div className="cl-head-art" aria-hidden="true">
            <AstridLogo animated tracking />
          </div>
          <div className="cl-head-copy">
            <p className="eyebrow">ASTRID / CHANGELOG</p>
            <h1>
              <VerseText text="What’s new?" tone="dark" seed={1115} />
            </h1>
            <p>Everything that has landed in Astrid, newest first.</p>
          </div>
        </header>
        <div className="cl-body">
          <nav className="cl-months" aria-label="Months">
            <b>2026</b>
            {MONTHS.map((m) => (
              <a key={m} href={`#${m}`} aria-current={m === month ? "true" : undefined}>
                <i />
                {fmt(`${m}-01`, { month: "long" })}
              </a>
            ))}
          </nav>
          <div className="cl-entries">
            {ENTRIES.map((entry, index) => {
              const firstOfMonth = ENTRIES.findIndex((x) => monthKey(x.date) === monthKey(entry.date)) === index;
              return (
                <article
                  key={entry.title}
                  className="cl-entry"
                  id={firstOfMonth ? monthKey(entry.date) : undefined}
                  data-month={firstOfMonth ? monthKey(entry.date) : undefined}
                >
                  {entry.mood && (
                    <span className="cl-mood" aria-hidden="true">
                      <AstridLogo expression={entry.mood[0]} />
                      <ThinkingOrb state={entry.mood[1]} size={32} theme="dark" />
                    </span>
                  )}
                  <time dateTime={entry.date}>
                    {fmt(entry.date, { month: "long", day: "numeric", year: "numeric" }).toUpperCase()}
                  </time>
                  <h2>{entry.title}</h2>
                  <span className={`cl-tag tag-${entry.tag.toLowerCase()}`}>{entry.tag}</span>
                  <p className="cl-lede">{entry.lede}</p>
                  <div className="cl-items">
                    {entry.items.map((item) => (
                      <div key={item.title} className="cl-item">
                        <h3>{item.title}</h3>
                        <p>{item.body}</p>
                        <a href={`${PR}${item.pr}`}>
                          Shipped in <span>#{item.pr}</span>
                        </a>
                      </div>
                    ))}
                  </div>
                </article>
              );
            })}
          </div>
        </div>
      </main>
      <Footer />
    </>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Changelog />
  </React.StrictMode>,
);
