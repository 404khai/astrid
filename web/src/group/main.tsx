import { StrictMode, useEffect, useRef, useState, type CSSProperties } from "react";
import { createRoot } from "react-dom/client";
import { FlatGoblin, LiveGoblin, type Item } from "../goblins/live";
import type { ExpressionId } from "@404khai/goblins/flat";
import "@fontsource/space-grotesk/latin-400.css";
import "@fontsource/space-grotesk/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import { AstridLogo } from "../components/AstridLogo";
import "./group.css";

/**
 * Goblin groups in the desktop app (cosmetic, ADR 0037 amendment). One page,
 * three modes picked by `?mode=`:
 * - `picker`: every goblin, pick some, see them as a group, make the group.
 * - `cluster`: a group's avatar, its goblins overlapping in a dark circle.
 * - `chat`: a mock chat between the group's goblins, with Astrid's mark up top.
 *   Its pill opens a side panel (name, description, goblins, save), and a
 *   goblin there opens its profile.
 * Goblins are drawn by the Goblins library (@404khai/goblins, MIT): live 3D
 * where they talk, flat SVG where they are small. Nothing here talks to a model.
 */
type Goblin = Item;
type Palette = { content: string; text: string; secondary: string; accent: string; bubble: string };
type Incoming =
  | { type: "picker"; items: Goblin[]; selected: string[]; palette?: Palette }
  | { type: "group"; name: string; about?: string; members: Goblin[]; palette?: Palette };

const host = window as unknown as {
  astrid?: { receive: (m: Incoming) => void };
  webkit?: { messageHandlers?: { astrid?: { postMessage: (m: unknown) => void } } };
};
const post = (m: unknown) => host.webkit?.messageHandlers?.astrid?.postMessage(m);
const mode = new URLSearchParams(location.search).get("mode") ?? "chat";
document.documentElement.dataset.mode = mode;

/** Goblins overlapping in a dark circle, as in the sidebar. */
function Cluster({ goblins, size }: { goblins: Goblin[]; size: number }) {
  const shown = goblins.slice(0, 3);
  const inner = shown.length === 1 ? size * 0.7 : size * 0.52;
  return (
    <span className="cluster" style={{ width: size, height: size } as CSSProperties} aria-label={goblins.map((b) => b.name).join(", ")}>
      {shown.map((b, i) => (
        <span key={b.id} className={`cluster-slot n${shown.length} s${i}`} style={{ width: inner, height: inner }}>
          <FlatGoblin item={b} size={inner} />
        </span>
      ))}
    </span>
  );
}

function usePalette(p?: Palette) {
  useEffect(() => {
    if (!p) return;
    const s = document.documentElement.style;
    s.setProperty("--content", p.content);
    s.setProperty("--text", p.text);
    s.setProperty("--secondary", p.secondary);
    s.setProperty("--accent", p.accent);
    s.setProperty("--bubble", p.bubble);
  }, [p]);
}

// ─── Picker ────────────────────────────────────────────────────────────────

function Picker() {
  const [items, setItems] = useState<Goblin[]>([]);
  const [picked, setPicked] = useState<string[]>([]);
  const [name, setName] = useState("");
  const [palette, setPalette] = useState<Palette>();
  usePalette(palette);
  useEffect(() => {
    host.astrid = {
      receive: (m) => {
        if (m.type !== "picker") return;
        setItems(m.items);
        setPicked(m.selected);
        if (m.palette) setPalette(m.palette);
      },
    };
    post({ type: "ready" });
  }, []);
  const chosen = picked.map((id) => items.find((b) => b.id === id)).filter(Boolean) as Goblin[];
  const toggle = (id: string) => {
    const next = picked.includes(id) ? picked.filter((p) => p !== id) : [...picked, id].slice(-10);
    setPicked(next);
    post({ type: "toggle", selected: next });
  };
  const placeholder = chosen.length === 0 ? "Pick some goblins" : chosen.length <= 2 ? chosen.map((b) => b.name).join(" & ") : `${chosen[0].name}, ${chosen[1].name} +${chosen.length - 2}`;
  return (
    <main className="picker">
      <header className="picker-top">
        <Cluster key={chosen.map((b) => b.id).join()} goblins={chosen} size={64} />
        <div className="picker-title">
          <strong>{chosen.length ? `${chosen.length} goblin${chosen.length === 1 ? "" : "s"}` : "Your goblins"}</strong>
          <span>{chosen.length ? chosen.map((b) => b.name).join(", ") : "Pick who joins the group"}</span>
        </div>
      </header>
      <div className="picker-grid">
        {items.map((b) => (
          <button key={b.id} type="button" className={`pick${picked.includes(b.id) ? " on" : ""}`} onClick={() => toggle(b.id)} aria-pressed={picked.includes(b.id)}>
            <FlatGoblin item={b} size={52} expression={b.character.expression} />
            <span>{b.name}</span>
          </button>
        ))}
      </div>
      <form
        className="picker-make"
        onSubmit={(e) => {
          e.preventDefault();
          if (chosen.length) post({ type: "create", name: name.trim() || placeholder, members: picked });
        }}
      >
        <input value={name} placeholder={placeholder} maxLength={60} onChange={(e) => setName(e.target.value)} aria-label="Group name" />
        <button type="submit" disabled={!chosen.length}>Make group</button>
      </form>
    </main>
  );
}

// ─── Cluster ───────────────────────────────────────────────────────────────

function ClusterPage() {
  const [members, setMembers] = useState<Goblin[]>([]);
  useEffect(() => {
    host.astrid = { receive: (m) => m.type === "group" && setMembers(m.members) };
    post({ type: "ready" });
  }, []);
  return <Cluster key={members.map((b) => b.id).join()} goblins={members} size={Math.min(innerWidth, innerHeight)} />;
}

// ─── Chat ──────────────────────────────────────────────────────────────────

type Line =
  | { kind: "text"; from: number; text: string; at: string; mention?: number; mood: ExpressionId }
  | { kind: "pulse"; from: number; at: string; mood: ExpressionId }
  | { kind: "activity"; from: number; at: string; mood: ExpressionId }
  | { kind: "you"; text: string; at: string }
  | { kind: "typing"; from: number };

/** A fixed little script, filled in with the group's own names. */
function script(n: number): Line[] {
  const at = (m: number) => `1:${String(40 + m).padStart(2, "0")} pm`;
  const lines: Line[] = [
    { kind: "text", from: 0, at: at(1), mood: "happy", text: "Morning all! Quick round before we start: where is everyone at?" },
    { kind: "text", from: 1 % n, at: at(2), mood: "talking", text: "Mine's nearly there, two small things left.", mention: n > 2 ? 2 : undefined },
    { kind: "pulse", from: 0, at: at(4), mood: "searching" },
    { kind: "you", at: at(5), text: "Looks good. Can someone take the last blocker today?" },
    { kind: "activity", from: (2 % n), at: at(6), mood: "thinking" },
    { kind: "text", from: (n > 3 ? 3 : 0), at: at(7), mood: "celebrate", text: "On it. I'll post an update in an hour." },
    { kind: "typing", from: n - 1 },
  ];
  return lines;
}

/** Fixed, seeded numbers so the mock looks the same every time. */
function seeded(seed: string, i: number) {
  let h = 2166136261;
  for (const c of seed + i) h = Math.imul(h ^ c.charCodeAt(0), 16777619);
  return ((h >>> 0) % 1000) / 1000;
}

function Pulse({ members }: { members: Goblin[] }) {
  const rows = members.slice(0, 5).map((b, i) => {
    const total = 4 + Math.round(seeded(b.id, i) * 6);
    const done = Math.round(total * (0.3 + seeded(b.id, i + 7) * 0.7));
    return { b, done, total };
  });
  const done = rows.reduce((s, r) => s + r.done, 0);
  const total = rows.reduce((s, r) => s + r.total, 0);
  const share = total ? done / total : 0;
  const r = 30, c = 2 * Math.PI * r;
  return (
    <div className="card pulse">
      <div className="card-head">
        <svg viewBox="0 0 80 80" className="ring" aria-label={`${Math.round(share * 100)} percent done`}>
          <circle cx="40" cy="40" r={r} className="ring-bg" />
          <circle cx="40" cy="40" r={r} className="ring-fg" strokeDasharray={`${c * share} ${c}`} />
          <text x="40" y="45" textAnchor="middle">{Math.round(share * 100)}%</text>
        </svg>
        <div>
          <strong>Today's pulse</strong>
          <span>{done} of {total} tasks done across {rows.length} goblins</span>
        </div>
      </div>
      <ul className="bars">
        {rows.map(({ b, done, total }) => (
          <li key={b.id}>
            <span className="bar-who">
              <FlatGoblin item={b} size={22} />
              {b.name}
            </span>
            <span className="bar"><span style={{ width: `${(done / total) * 100}%` }} /></span>
            <span className="bar-n">{done}/{total}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

function Activity({ seed, names }: { seed: string; names: string[] }) {
  const points = Array.from({ length: 9 }, (_, i) => 0.15 + seeded(seed, i) * 0.8);
  const w = 300, h = 80;
  const xy = points.map((p, i) => [(i / (points.length - 1)) * w, h - p * h] as const);
  const line = xy.map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(1)} ${y.toFixed(1)}`).join(" ");
  return (
    <div className="card activity">
      <strong>Messages, last 8 hours</strong>
      <svg viewBox={`0 0 ${w} ${h + 4}`} className="spark" preserveAspectRatio="none" aria-hidden="true">
        <path d={`${line} L${w} ${h} L0 ${h} Z`} className="spark-fill" />
        <path d={line} className="spark-line" />
        {xy.map(([x, y], i) => (i === xy.length - 1 ? <circle key={i} cx={x} cy={y} r="3.5" className="spark-dot" /> : null))}
      </svg>
      <div className="chips">
        <span className="chip ok">2 shipped</span>
        <span className="chip wait">1 waiting on {names[1 % names.length]}</span>
        <span className="chip block">1 blocker</span>
      </div>
    </div>
  );
}

const FACE = 64;

/** Up to three live goblins, overlapping: the group's mark in its panel. */
function GroupMark({ members }: { members: Goblin[] }) {
  const shown = members.slice(0, 3);
  return (
    <div className={`group-mark n${shown.length}`} aria-hidden="true">
      {shown.map((b, i) => (
        <span key={b.id} className={`mark-slot s${i}`}>
          <LiveGoblin item={b} size={shown.length === 1 ? 120 : 96} />
        </span>
      ))}
    </div>
  );
}

const TRAITS: [string, (c: Goblin["character"]) => unknown][] = [
  ["Head", (c) => c.head],
  ["Ears", (c) => c.ears],
  ["Eyes", (c) => c.eyes],
  ["Nose", (c) => c.nose],
  ["Hat", (c) => (c.hat && typeof c.hat === "object" ? (c.hat as { style?: string }).style : c.hat)],
  ["Beard", (c) => c.beard],
  ["Eyewear", (c) => c.eyewear],
  ["Scarf", (c) => c.scarf],
];

/** A goblin's profile: big and live (click it for its expressions), with its traits. */
function Profile({ goblin, onClose }: { goblin: Goblin; onClose: () => void }) {
  useEffect(() => {
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    addEventListener("keydown", key);
    return () => removeEventListener("keydown", key);
  }, [onClose]);
  const c = goblin.character;
  const traits = TRAITS.map(([k, f]) => [k, f(c)] as const).filter(([, v]) => typeof v === "string" && v && v !== "none");
  return (
    <div className="scrim" onClick={onClose}>
      <div className="profile" role="dialog" aria-modal="true" aria-label={`${goblin.name}'s profile`} onClick={(e) => e.stopPropagation()}>
        <button type="button" className="close" onClick={onClose} aria-label="Close">✕</button>
        <LiveGoblin item={goblin} size={180} framing="full" title="Click for the next expression" />
        <h2>{goblin.name}</h2>
        <span className="skin"><i style={{ background: String(c.skin ?? "#888") }} />{String(c.skin ?? "")}</span>
        <dl>
          {traits.map(([k, v]) => (
            <div key={k}><dt>{k}</dt><dd>{String(v)}</dd></div>
          ))}
        </dl>
      </div>
    </div>
  );
}

/** The group's side panel, from the right. */
function Panel({ name, about, members, open, onClose, onPick }: { name: string; about: string; members: Goblin[]; open: boolean; onClose: () => void; onPick: (g: Goblin) => void }) {
  const [draft, setDraft] = useState({ name, about });
  useEffect(() => setDraft({ name, about }), [name, about, open]);
  const dirty = draft.name.trim() !== name || draft.about.trim() !== about;
  return (
    <>
      <div className={`panel-scrim${open ? " open" : ""}`} onClick={onClose} />
      <aside className={`panel${open ? " open" : ""}`} aria-hidden={!open} aria-label="Group">
        <button type="button" className="close" onClick={onClose} aria-label="Close panel" tabIndex={open ? 0 : -1}>✕</button>
        {open && <GroupMark members={members} />}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (draft.name.trim()) post({ type: "save", name: draft.name.trim(), about: draft.about.trim() });
          }}
        >
          <label>
            <span>Name</span>
            <input value={draft.name} maxLength={60} onChange={(e) => setDraft({ ...draft, name: e.target.value })} tabIndex={open ? 0 : -1} />
          </label>
          <label>
            <span>Description</span>
            <textarea value={draft.about} maxLength={280} rows={3} placeholder="What is this group for?" onChange={(e) => setDraft({ ...draft, about: e.target.value })} tabIndex={open ? 0 : -1} />
          </label>
          <span className="panel-label">{members.length} goblin{members.length === 1 ? "" : "s"}</span>
          <ul className="roster">
            {members.map((b) => (
              <li key={b.id}>
                <button type="button" onClick={() => onPick(b)} tabIndex={open ? 0 : -1}>
                  <i className="roster-face"><FlatGoblin item={b} size={36} /></i>
                  <span>{b.name}</span>
                  <em>›</em>
                </button>
              </li>
            ))}
          </ul>
          <button type="submit" className="save" disabled={!dirty || !draft.name.trim()} tabIndex={open ? 0 : -1}>Save</button>
        </form>
      </aside>
    </>
  );
}

function Chat() {
  const [name, setName] = useState("");
  const [about, setAbout] = useState("");
  const [members, setMembers] = useState<Goblin[]>([]);
  const [palette, setPalette] = useState<Palette>();
  const [panel, setPanel] = useState(() => new URLSearchParams(location.search).has("panel"));
  const [profile, setProfile] = useState<Goblin | null>(null);
  usePalette(palette);
  useEffect(() => {
    host.astrid = {
      receive: (m) => {
        if (m.type !== "group") return;
        setName(m.name);
        setAbout(m.about ?? "");
        setMembers(m.members);
        if (m.palette) setPalette(m.palette);
      },
    };
    post({ type: "ready" });
  }, []);
  if (!members.length) return null;
  const n = members.length;
  const who = (i: number) => members[i % n];
  return (
    <main className="chat">
      <button type="button" className="chat-settings" onClick={() => setPanel(true)} aria-label="Group settings" title="Group settings">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 15.2a3.2 3.2 0 1 0 0-6.4 3.2 3.2 0 0 0 0 6.4Z" /><path d="M19.4 13.5a7.7 7.7 0 0 0 0-3l2-1.6-2-3.4-2.4 1a7.6 7.6 0 0 0-2.6-1.5L14 2.5h-4l-.4 2.5A7.6 7.6 0 0 0 7 6.5l-2.4-1-2 3.4 2 1.6a7.7 7.7 0 0 0 0 3l-2 1.6 2 3.4 2.4-1a7.6 7.6 0 0 0 2.6 1.5l.4 2.5h4l.4-2.5a7.6 7.6 0 0 0 2.6-1.5l2.4 1 2-3.4-2-1.6Z" /></svg>
      </button>
      <header className="chat-top">
        {/* The app's card header shows the group's name; the page keeps the mark and the count. */}
        <div className="chat-center">
          <span className="chat-logo"><AstridLogo animated tracking /></span>
          <button type="button" className="chat-count" onClick={() => setPanel(true)} aria-expanded={panel} aria-label={`${n} goblins. Open the group panel.`}>
            <Cluster key={members.map((b) => b.id).join()} goblins={members} size={26} />
            {n} goblin{n === 1 ? "" : "s"}
          </button>
        </div>
      </header>
      <section className="lines">
        {script(n).map((line, i) => {
          if (line.kind === "you")
            return (
              <div key={i} className="line you">
                <p className="bubble">{line.text}</p>
                <time>{line.at}</time>
              </div>
            );
          const b = who(line.from);
          if (line.kind === "typing")
            return (
              <div key={i} className="line">
                <span className="face"><LiveGoblin item={b} size={FACE} expression="thinking" /></span>
                <p className="bubble typing" aria-label={`${b.name} is typing`}><i /><i /><i /></p>
              </div>
            );
          return (
            <div key={i} className="line">
              <span className="face"><LiveGoblin item={b} size={FACE} expression={line.mood} /></span>
              <div className="said">
                <span className="who">{b.name}</span>
                {line.kind === "text" && (
                  <p className="bubble">
                    {line.text}
                    {line.mention !== undefined && <> <b className="mention">@{who(line.mention).name}</b> can you look at the rest?</>}
                  </p>
                )}
                {line.kind === "pulse" && <Pulse members={members} />}
                {line.kind === "activity" && <Activity seed={name} names={members.map((m) => m.name)} />}
                <time>{line.at}</time>
              </div>
            </div>
          );
        })}
      </section>
      <footer className="chat-compose" aria-hidden="true">
        <span>Message {name}…</span>
        <span className="send">↑</span>
      </footer>
      <Panel name={name} about={about} members={members} open={panel} onClose={() => setPanel(false)} onPick={setProfile} />
      {profile && <Profile goblin={profile} onClose={() => setProfile(null)} />}
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>{mode === "picker" ? <Picker /> : mode === "cluster" ? <ClusterPage /> : <Chat />}</StrictMode>,
);

// Dev only (vite, no app): a fake group so the page can be designed in a browser.
if (!host.webkit?.messageHandlers?.astrid) {
  Promise.all(["06-sunee", "07-kirby", "08-cloudee", "03-citrus"].map((f) => fetch(`/goblins/${f}.goblin.json`).then((r) => r.json()).catch(() => null))).then((cs) => {
    const items = cs.map((c, i) => (c ? { id: `b${i}`, name: c.name ?? `Goblin ${i}`, character: c } : null)).filter(Boolean) as Goblin[];
    setTimeout(() => host.astrid?.receive(mode === "picker" ? { type: "picker", items, selected: [] } : { type: "group", name: "Launch week", members: items }), 50);
  });
}
