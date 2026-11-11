import {
  Add01Icon,
  AiBrain01Icon,
  ArrowLeft01Icon,
  ArrowRight01Icon,
  ArrowUp02Icon,
  Book02Icon,
  BubbleChatAddIcon,
  CubeIcon,
  Cursor01Icon,
  DrawingModeIcon,
  EyeIcon,
  Home01Icon,
  LaptopIcon,
  Loading03Icon,
  Message01Icon,
  Mic01Icon,
  Shield01Icon,
  SidebarLeftIcon,
  SmartPhone01Icon,
  Tick02Icon,
  UnfoldMoreIcon,
  WorkflowCircle01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import { AstridLogo } from "./AstridLogo";
import { DitherField } from "./DitherField";
import { VerseText } from "./VerseText";

function Glyph({ icon, size = 16 }: { icon: IconSvgElement; size?: number }) {
  return <HugeiconsIcon icon={icon} size={size} strokeWidth={1.6} aria-hidden="true" />;
}

const tabs = [
  "Explain the engine figure",
  "Walk me through SSH keys",
  "Why is the build red?",
  "Draw the auth flow",
  "Router behind the TV",
];

const astrix = [
  { name: "Figures", icon: CubeIcon, tone: "#8793ff" },
  { name: "Sketchpad", icon: DrawingModeIcon, tone: "#30a46c" },
  { name: "Guide", icon: Cursor01Icon, tone: "#f7c600" },
  { name: "Reader", icon: Book02Icon, tone: "#ec6a8a" },
];

const recent: [string, string, "running" | "done" | "waiting"][] = [
  ["Explain the engine figure", "29s", "running"],
  ["Walk me through SSH keys", "4m", "waiting"],
  ["Why is the build red?", "12m", "done"],
  ["Draw the auth flow", "1h", "done"],
  ["Router behind the TV", "3h", "done"],
  ["Read chapter 4 with me", "1d", "done"],
  ["Compare these two plans", "2d", "done"],
];

const greeting =
  "Morning, Khai. Preview is open on chapter 4... want me to build the engine and walk you through it?";

/** Astrid's home window: a Hoplite-style shell around a living dither scene,
 * with Astrid hanging on its thread, writing. */
export function AstridHome() {
  return (
    <div className="home-stage">
      <div className="home-window" role="img" aria-label="Illustration: the Astrid desktop app home screen">
        <aside className="home-sidebar">
          <div className="home-chrome">
            <span className="traffic">
              <i />
              <i />
              <i />
            </span>
            <span className="home-chrome-tools">
              <Glyph icon={ArrowLeft01Icon} />
              <Glyph icon={ArrowRight01Icon} />
              <Glyph icon={SidebarLeftIcon} />
            </span>
          </div>
          <nav className="home-primary">
            <span className="active">
              <Glyph icon={BubbleChatAddIcon} /> New ask
            </span>
            <span>
              <Glyph icon={Message01Icon} /> Sessions
            </span>
            <span>
              <Glyph icon={WorkflowCircle01Icon} /> Routines
            </span>
          </nav>
          <div className="home-group">
            <span className="home-group-title">
              Astrix <Glyph icon={Add01Icon} size={14} />
            </span>
            {astrix.map(({ name, icon, tone }) => (
              <span key={name} className="home-astrix">
                <i style={{ color: tone }}>
                  <Glyph icon={icon} size={14} />
                </i>
                {name}
              </span>
            ))}
          </div>
          <div className="home-recent">
            {recent.map(([title, age, state]) => (
              <span key={title} data-state={state}>
                <i>
                  <Glyph icon={state === "done" ? Tick02Icon : Loading03Icon} size={14} />
                </i>
                <b>{title}</b>
                {state === "waiting" ? <em>Your turn</em> : null}
                <small>{age}</small>
              </span>
            ))}
          </div>
          <div className="home-account">
            <span className="home-avatar">
              <AstridLogo />
            </span>
            <span>
              <b>Khai</b>
              <small>Personal · MacBook Pro</small>
            </span>
            <Glyph icon={UnfoldMoreIcon} />
          </div>
        </aside>
        <div className="home-main">
          <div className="home-tabs">
            <span className="active">
              <Glyph icon={Home01Icon} size={14} /> Home
            </span>
            {tabs.map((tab, index) => (
              <span key={tab}>
                <i data-state={index === 0 ? "running" : index === 1 ? "waiting" : "done"} />
                {tab}
              </span>
            ))}
            <Glyph icon={Add01Icon} size={14} />
          </div>
          <div className="home-canvas">
            <DitherField />
            <div className="home-thread" aria-hidden="true">
              <span className="thread-line" />
              <span className="thread-logo">
                <AstridLogo animated />
              </span>
            </div>
            <div className="home-writing">
              <VerseText text={greeting} tone="dark" seed={404} className="home-verse" />
            </div>
            <div className="home-composer">
              <div className="composer-box">
                <span className="composer-placeholder">
                  Ask Astrid, or hold ⌃ ⌥ to talk…
                </span>
                <div className="composer-row">
                  <Glyph icon={Add01Icon} />
                  <span className="composer-chip">
                    <Glyph icon={AiBrain01Icon} size={14} /> Auto model
                  </span>
                  <span className="composer-mic">
                    <Glyph icon={Mic01Icon} />
                  </span>
                </div>
              </div>
              <div className="composer-footer">
                <span>
                  <Glyph icon={LaptopIcon} size={14} /> MacBook Pro
                </span>
                <span className="muted">
                  <Glyph icon={SmartPhone01Icon} size={14} /> iPhone · soon
                </span>
                <span className="push">
                  <Glyph icon={EyeIcon} size={14} /> Look: ask each time
                </span>
                <span>
                  <Glyph icon={Shield01Icon} size={14} /> On device
                </span>
                <span className="composer-send">
                  <Glyph icon={ArrowUp02Icon} />
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
