import { StrictMode, useEffect, useRef, useState, type CSSProperties } from "react";
import { createRoot } from "react-dom/client";
import { createAvatar } from "@bible-strong/avatar-web";
import "@fontsource/space-grotesk/latin-400.css";
import "@fontsource/space-grotesk/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import { AstridLogo } from "../components/AstridLogo";
import "./group.css";

/**
 * Blob groups in the desktop app (cosmetic, ADR 0037 amendment). One page,
 * three modes picked by `?mode=`:
 * - `picker`: every blob, pick some, see them as a group, make the group.
 * - `cluster`: a group's avatar, its blobs overlapping in a dark circle.
 * - `chat`: a mock chat between the group's blobs, with Astrid's mark up top.
 * Blobs are drawn by the MIT `@bible-strong/avatar-web` renderer from the
 * characters the app sends. Nothing here talks to a model.
 */
type Blob = { id: string; name: string; character: unknown };
type Palette = { content: string; text: string; secondary: string; accent: string; bubble: string };
type Incoming =
  | { type: "picker"; items: Blob[]; selected: string[]; palette?: Palette }
  | { type: "group"; name: string; members: Blob[]; palette?: Palette };

const host = window as unknown as {
  astrid?: { receive: (m: Incoming) => void };
  webkit?: { messageHandlers?: { astrid?: { postMessage: (m: unknown) => void } } };
};
const post = (m: unknown) => host.webkit?.messageHandlers?.astrid?.postMessage(m);
const mode = new URLSearchParams(location.search).get("mode") ?? "chat";
document.documentElement.dataset.mode = mode;

/** One live blob. `state` is a state of the renderer's catalogue. */
function BlobView({ blob, size, state, follow = false, settle = 0 }: { blob: Blob; size: number; state?: string; follow?: boolean; settle?: number }) {
  const ref = useRef<HTMLSpanElement>(null);
  const avatar = useRef<ReturnType<typeof createAvatar> | null>(null);
  useEffect(() => {
    if (!ref.current) return;
    try {
      avatar.current = createAvatar(ref.current, { character: blob.character, size, followPointer: follow, settleAfterMs: settle, ariaLabel: blob.name });
    } catch {
      avatar.current = null;
    }
    return () => avatar.current?.destroy();
  }, [blob.id, size]);
  useEffect(() => {
    if (state) avatar.current?.setState(state);
  }, [state]);
  return <span className="blob" ref={ref} onClick={() => avatar.current?.poke("bounce")} />;
}

/** Blobs overlapping in a dark circle, as in the sidebar. */
function Cluster({ blobs, size }: { blobs: Blob[]; size: number }) {
  const shown = blobs.slice(0, 3);
  const inner = shown.length === 1 ? size * 0.7 : size * 0.52;
  return (
    <span className="cluster" style={{ width: size, height: size } as CSSProperties} aria-label={blobs.map((b) => b.name).join(", ")}>
      {shown.map((b, i) => (
        <span key={b.id} className={`cluster-slot n${shown.length} s${i}`} style={{ width: inner, height: inner }}>
          <BlobView blob={b} size={inner} settle={4000} />
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
  const [items, setItems] = useState<Blob[]>([]);
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
  const chosen = picked.map((id) => items.find((b) => b.id === id)).filter(Boolean) as Blob[];
  const toggle = (id: string) => {
    const next = picked.includes(id) ? picked.filter((p) => p !== id) : [...picked, id].slice(-10);
    setPicked(next);
    post({ type: "toggle", selected: next });
  };
  const placeholder = chosen.length === 0 ? "Pick some blobs" : chosen.length <= 2 ? chosen.map((b) => b.name).join(" & ") : `${chosen[0].name}, ${chosen[1].name} +${chosen.length - 2}`;
  return (
    <main className="picker">
      <header className="picker-top">
        <Cluster key={chosen.map((b) => b.id).join()} blobs={chosen} size={64} />
        <div className="picker-title">
          <strong>{chosen.length ? `${chosen.length} blob${chosen.length === 1 ? "" : "s"}` : "Your blobs"}</strong>
          <span>{chosen.length ? chosen.map((b) => b.name).join(", ") : "Pick who joins the group"}</span>
        </div>
      </header>
      <div className="picker-grid">
        {items.map((b) => (
          <button key={b.id} type="button" className={`pick${picked.includes(b.id) ? " on" : ""}`} onClick={() => toggle(b.id)} aria-pressed={picked.includes(b.id)}>
            <BlobView blob={b} size={46} settle={5000} follow />
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
  const [members, setMembers] = useState<Blob[]>([]);
  useEffect(() => {
    host.astrid = { receive: (m) => m.type === "group" && setMembers(m.members) };
    post({ type: "ready" });
  }, []);
  return <Cluster key={members.map((b) => b.id).join()} blobs={members} size={Math.min(innerWidth, innerHeight)} />;
}

// ─── Chat ──────────────────────────────────────────────────────────────────

type Line =
  | { kind: "text"; from: number; text: string; at: string; mention?: number }
  | { kind: "pulse"; from: number; at: string }
  | { kind: "activity"; from: number; at: string }
  | { kind: "you"; text: string; at: string }
  | { kind: "typing"; from: number };

/** A fixed little script, filled in with the group's own names. */
function script(n: number): Line[] {
  const at = (m: number) => `1:${String(40 + m).padStart(2, "0")} pm`;
  const lines: Line[] = [
    { kind: "text", from: 0, at: at(1), text: "Morning all! Quick round before we start: where is everyone at?" },
    { kind: "text", from: 1 % n, at: at(2), text: "Mine's nearly there, two small things left.", mention: n > 2 ? 2 : undefined },
    { kind: "pulse", from: 0, at: at(4) },
    { kind: "you", at: at(5), text: "Looks good. Can someone take the last blocker today?" },
    { kind: "activity", from: (2 % n), at: at(6) },
    { kind: "text", from: (n > 3 ? 3 : 0), at: at(7), text: "On it. I'll post an update in an hour." },
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

function Pulse({ members }: { members: Blob[] }) {
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
          <span>{done} of {total} tasks done across {rows.length} blobs</span>
        </div>
      </div>
      <ul className="bars">
        {rows.map(({ b, done, total }) => (
          <li key={b.id}>
            <span className="bar-who">
              <BlobView blob={b} size={20} settle={3000} />
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

function Chat() {
  const [name, setName] = useState("");
  const [members, setMembers] = useState<Blob[]>([]);
  const [palette, setPalette] = useState<Palette>();
  usePalette(palette);
  useEffect(() => {
    host.astrid = {
      receive: (m) => {
        if (m.type !== "group") return;
        setName(m.name);
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
      <header className="chat-top">
        {/* The app's card header shows the group's name; the page keeps the mark and the count. */}
        <div className="chat-center">
          <span className="chat-logo"><AstridLogo animated tracking /></span>
          <span className="chat-count">
            <Cluster key={members.map((b) => b.id).join()} blobs={members} size={26} />
            {n} blob{n === 1 ? "" : "s"}
          </span>
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
                <span className="face"><BlobView blob={b} size={44} state="thinking" follow /></span>
                <p className="bubble typing" aria-label={`${b.name} is typing`}><i /><i /><i /></p>
              </div>
            );
          return (
            <div key={i} className="line">
              <span className="face"><BlobView blob={b} size={44} settle={6000} follow /></span>
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
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>{mode === "picker" ? <Picker /> : mode === "cluster" ? <ClusterPage /> : <Chat />}</StrictMode>,
);

// Dev only (vite, no app): a fake group so the page can be designed in a browser.
if (!host.webkit?.messageHandlers?.astrid) {
  Promise.all(["06-sunee", "07-kirby", "08-cloudee", "03-citrus"].map((f) => fetch(`/blobs/${f}.avatar.json`).then((r) => r.json()).catch(() => null))).then((cs) => {
    const items = cs.map((c, i) => (c ? { id: `b${i}`, name: c.name ?? `Blob ${i}`, character: c } : null)).filter(Boolean) as Blob[];
    setTimeout(() => host.astrid?.receive(mode === "picker" ? { type: "picker", items, selected: [] } : { type: "group", name: "Launch week", members: items }), 50);
  });
}
