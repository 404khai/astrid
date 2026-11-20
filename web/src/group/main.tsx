import { StrictMode, useEffect, useRef, useState, type CSSProperties } from "react";
import { createRoot } from "react-dom/client";
import { FlatGoblin, LiveGoblin, type Item } from "../goblins/live";
import type { ExpressionId } from "@404khai/goblins/flat";
import "@fontsource/space-grotesk/latin-400.css";
import "@fontsource/space-grotesk/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import { AstridLogo, type EyeExpression } from "../components/AstridLogo";
import "./group.css";
import { InlineChart, checkChart } from "./Chart";

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
  | { type: "group"; name: string; about?: string; members: Goblin[]; palette?: Palette; messages?: Message[]; roles?: Record<string, string>; astrid?: Overseer; panel?: number; memory?: Record<string, string[]>; live?: { speaker: string; text: string } };
type Overseer = "idle" | "tagged" | "question" | "checking" | "talking";
type Message = { id: string; author: string; text: string; at: string; mood: ExpressionId; mentions: string[]; reactions: { emoji: string; by: string[] }[]; outcome?: string | null; chart?: unknown };

/** Astrid's eyes for what it is doing in the group. */
const EYES: Record<Overseer, EyeExpression | undefined> = { idle: undefined, tagged: "listening", question: "solving", checking: "searching", talking: "composing" };
const QUICK = ["👍", "❤️", "😂", "🎉", "👀", "🤔"];

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

const FACE = 84;

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
function Panel({ name, about, members, roles, memory, open, onClose, onPick }: { name: string; about: string; members: Goblin[]; roles: Record<string, string>; memory: Record<string, string[]>; open: boolean; onClose: () => void; onPick: (g: Goblin) => void }) {
  const [draft, setDraft] = useState({ name, about, roles });
  useEffect(() => setDraft({ name, about, roles }), [name, about, roles, open]);
  const dirty = draft.name.trim() !== name || draft.about.trim() !== about || members.some((b) => (draft.roles[b.id] ?? "").trim() !== (roles[b.id] ?? ""));
  return (
    <>
      <div className={`panel-scrim${open ? " open" : ""}`} onClick={onClose} />
      <aside className={`panel${open ? " open" : ""}`} aria-hidden={!open} aria-label="Group">
        <button type="button" className="close" onClick={onClose} aria-label="Close panel" tabIndex={open ? 0 : -1}>✕</button>
        {open && <GroupMark members={members} />}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (draft.name.trim()) post({ type: "save", name: draft.name.trim(), about: draft.about.trim(), roles: Object.fromEntries(members.map((b) => [b.id, (draft.roles[b.id] ?? "").trim()])) });
          }}
        >
          <label>
            <span>Name</span>
            <input value={draft.name} maxLength={60} onChange={(e) => setDraft({ ...draft, name: e.target.value })} tabIndex={open ? 0 : -1} />
          </label>
          <label>
            <span>Description</span>
            <textarea value={draft.about} maxLength={280} rows={3} placeholder="What is this group for? Every goblin reads this." onChange={(e) => setDraft({ ...draft, about: e.target.value })} tabIndex={open ? 0 : -1} />
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
                <input className="role" value={draft.roles[b.id] ?? ""} maxLength={120} placeholder={`${b.name}'s role, e.g. researcher`} aria-label={`${b.name}'s role`} tabIndex={open ? 0 : -1} onChange={(e) => setDraft({ ...draft, roles: { ...draft.roles, [b.id]: e.target.value } })} />
                {(memory[b.id]?.length ?? 0) > 0 && (
                  <details className="memory">
                    <summary>{memory[b.id].length} thing{memory[b.id].length === 1 ? "" : "s"} remembered</summary>
                    <ul>{memory[b.id].slice(-8).map((f, i) => <li key={i}>{f}</li>)}</ul>
                    <button type="button" className="forget" onClick={() => post({ type: "forget", goblin: b.id })}>Forget all</button>
                  </details>
                )}
              </li>
            ))}
          </ul>
          <button type="submit" className="save" disabled={!dirty || !draft.name.trim()} tabIndex={open ? 0 : -1}>Save</button>
        </form>
      </aside>
    </>
  );
}

/** Text with @mentions picked out, each with its goblin beside it (every
 * goblin for @all, Astrid's mark for @astrid); tapping one puts it in the composer. */
function Said({ text, members }: { text: string; members: Goblin[] }) {
  const all = [...members.map((b) => b.name), "astrid", "all", "everyone"].sort((a, b) => b.length - a.length).map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const parts = text.split(new RegExp(`(?<![\\p{L}\\p{N}])(@(?:${all.join("|")}))(?![\\p{L}\\p{N}])`, "giu"));
  return (
    <>
      {parts.map((p, i) => {
        if (i % 2 === 0) return p;
        const name = p.slice(1).toLowerCase();
        const who = name === "all" || name === "everyone" ? members : members.filter((b) => b.name.toLowerCase() === name);
        const face = name === "astrid" ? <span className="tag-face astrid"><AstridLogo /></span> : who.length ? <span className={`tag-face n${Math.min(who.length, 3)}`}>{who.slice(0, 3).map((b) => <FlatGoblin key={b.id} item={b} size={20} />)}</span> : null;
        return <b key={i} className="mention" onClick={() => post({ type: "mention", name: p.slice(1) })}>{face}{p}</b>;
      })}
    </>
  );
}

/** Reactions under a bubble, as in the Reaction Bubble design: a dark bubble
 * with the emoji and a small bubble with who reacted. */
function Reactions({ m, face }: { m: Message; face: (key: string, size: number) => React.ReactNode }) {
  if (!m.reactions.length) return null;
  return (
    <div className="reactions">
      {m.reactions.map((r) => (
        <button key={r.emoji} type="button" className={`reaction${r.by.includes("you") ? " mine" : ""}`} onClick={() => post({ type: "react", id: m.id, emoji: r.emoji })} title={`${r.emoji} ${r.by.length}`}>
          <span className="reaction-emoji">{r.emoji}{r.by.length > 1 && <small>{r.by.length}</small>}</span>
          <span className="reaction-who">{r.by.slice(0, 2).map((k) => <i key={k}>{face(k, 16)}</i>)}</span>
        </button>
      ))}
    </div>
  );
}

function Chat() {
  const [g, setG] = useState<Extract<Incoming, { type: "group" }>>();
  const [panel, setPanel] = useState(() => new URLSearchParams(location.search).has("panel"));
  const [profile, setProfile] = useState<Goblin | null>(null);
  const [pop, setPop] = useState(false);
  const lastPanel = useRef<number | undefined>(undefined);
  const end = useRef<HTMLDivElement>(null);
  usePalette(g?.palette);
  useEffect(() => {
    host.astrid = { receive: (m) => m.type === "group" && setG(m) };
    post({ type: "ready" });
  }, []);
  useEffect(() => {
    if (g?.panel === undefined) return;
    if (lastPanel.current !== undefined && g.panel !== lastPanel.current) setPanel(true);
    lastPanel.current = g.panel;
  }, [g?.panel]);
  useEffect(() => {
    if (g?.astrid !== "tagged") return;
    setPop(true);
    const t = setTimeout(() => setPop(false), 700);
    return () => clearTimeout(t);
  }, [g?.astrid]);
  const count = g?.messages?.length ?? 0;
  const liveText = g?.live?.text ?? "";
  useEffect(() => { end.current?.scrollIntoView({ block: "end", behavior: "smooth" }); }, [count, liveText.length > 0, g?.live?.speaker]);
  if (!g || !g.members.length) return null;
  const { members, name } = g;
  const messages = g.messages ?? [];
  const n = members.length;
  const byKey = (k: string) => members.find((b) => b.id === k);
  const nameOf = (k: string) => (k === "you" ? "You" : k === "astrid" ? "Astrid" : byKey(k)?.name ?? "Goblin");
  const face = (k: string, size: number) => {
    if (k === "astrid") return <span className="mini-astrid" style={{ width: size, height: size }}><AstridLogo /></span>;
    if (k === "you") return <span className="mini-you" style={{ width: size, height: size }} />;
    const b = byKey(k);
    return b ? <FlatGoblin item={b} size={size} /> : null;
  };
  const avatar = (k: string, mood: ExpressionId) =>
    k === "astrid" ? <span className="astrid-face"><AstridLogo animated expression={mood === "thinking" ? "searching" : undefined} /></span> : byKey(k) ? <LiveGoblin item={byKey(k)!} size={FACE} expression={mood} /> : null;
  const live = g.live;
  return (
    <main className="chat">
      <header className="chat-top">
        <div className="chat-center">
          <span className={`chat-logo${pop ? " pop" : ""}`} title={`Astrid: ${g.astrid ?? "idle"}`}><AstridLogo animated tracking={!g.astrid || g.astrid === "idle"} expression={EYES[g.astrid ?? "idle"]} /></span>
          <button type="button" className="chat-count" onClick={() => setPanel(true)} aria-expanded={panel} aria-label={`${n} goblins. Open the group panel.`}>
            <Cluster key={members.map((b) => b.id).join()} goblins={members} size={26} />
            {n} goblin{n === 1 ? "" : "s"}
          </button>
        </div>
      </header>
      <section className="lines">
        {!messages.length && !live && (
          <div className="empty">
            <strong>Say hi to {name}</strong>
            <span>Tag a goblin with @ to ask them, or just talk: Astrid watches over the group and hands work out.</span>
          </div>
        )}
        {messages.map((m) => {
          const picks = (
            <span className="quick" role="group" aria-label="React">
              {QUICK.map((e) => <button key={e} type="button" onClick={() => post({ type: "react", id: m.id, emoji: e })}>{e}</button>)}
            </span>
          );
          if (m.author === "you")
            return (
              <div key={m.id} className="line you">
                <div className="said">
                  <p className="bubble"><Said text={m.text} members={members} />{picks}</p>
                  <Reactions m={m} face={face} />
                  <time>{m.at}</time>
                </div>
              </div>
            );
          return (
            <div key={m.id} className={`line${m.author === "astrid" ? " astrid" : ""}`}>
              <span className="face">{avatar(m.author, m.mood)}</span>
              <div className="said">
                <span className="who">{nameOf(m.author)}{m.author !== "astrid" && g.roles?.[m.author] ? <em> · {g.roles[m.author]}</em> : null}</span>
                <p className={`bubble${m.outcome ? " failed" : ""}`}><Said text={m.text} members={members} />{picks}</p>
                {(() => { const spec = checkChart(m.chart); return spec ? <InlineChart spec={spec} /> : null; })()}
                <Reactions m={m} face={face} />
                <time>{m.at}</time>
              </div>
            </div>
          );
        })}
        {live && (
          <div className={`line${live.speaker === "astrid" ? " astrid" : ""}`}>
            <span className="face">{avatar(live.speaker, live.text ? "talking" : "thinking")}</span>
            <div className="said">
              <span className="who">{nameOf(live.speaker)}</span>
              {live.text ? <p className="bubble"><Said text={live.text} members={members} /></p> : <p className="bubble typing" aria-label={`${nameOf(live.speaker)} is typing`}><i /><i /><i /></p>}
            </div>
          </div>
        )}
        <div ref={end} />
      </section>
      <Panel name={name} about={g.about ?? ""} members={members} roles={g.roles ?? {}} memory={g.memory ?? {}} open={panel} onClose={() => setPanel(false)} onPick={setProfile} />
      {profile && <Profile goblin={profile} onClose={() => setProfile(null)} />}
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>{mode === "picker" ? <Picker /> : mode === "cluster" ? <ClusterPage /> : <Chat />}</StrictMode>,
);

// Dev only (vite, no app): a fake group so the page can be designed in a browser.
if (!host.webkit?.messageHandlers?.astrid) {
  Promise.all(["04-pip", "03-bramble", "01-grub"].map((f) => fetch(`/goblins/${f}.goblin.json`).then((r) => r.json()).then((j) => j.goblins?.[0] ?? j).catch(() => null))).then((cs) => {
    const items = cs.map((c, i) => (c ? { id: `b${i}`, name: c.name ?? `Goblin ${i}`, character: c } : null)).filter(Boolean) as Goblin[];
    setTimeout(() => host.astrid?.receive(mode === "picker" ? { type: "picker", items, selected: [] } : { type: "group", name: "Launch week", members: items, astrid: "checking", messages: [
      { id: "1", author: "you", text: "@Sunee can you check the launch copy?", at: "1:41 pm", mood: "idle", mentions: ["b0"], reactions: [{ emoji: "👍", by: ["b0", "astrid"] }] },
      { id: "2", author: "b0", text: "On it! @Kirby, want to take the screenshots?", at: "1:42 pm", mood: "happy", mentions: ["b1"], reactions: [] },
    ], live: { speaker: "b1", text: "" } }), 50);
  });
}
