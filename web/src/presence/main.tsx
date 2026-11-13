import { StrictMode, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource/space-grotesk/latin-400.css";
import "@fontsource/space-grotesk/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import { Liquid } from "liquid-gooey";
import { ThinkingOrb } from "thinking-orbs";
import type { OrbState } from "thinking-orbs";
import { HugeiconsIcon } from "@hugeicons/react";
import { ArrowUp02Icon, Mail01Icon, Mic01Icon } from "@hugeicons/core-free-icons";
import { VoiceRing, useMicrophone } from "voice-ring";
import { AstridLogo, type EyeExpression } from "../components/AstridLogo";
import { listen, native, send, type Brief, type Incoming } from "./bridge";
import "./presence.css";

/** Runtime states → the docs' agent states (thinking orb + eye expression). */
const looks: Record<string, { orb: OrbState; eyes: EyeExpression }> = {
  working: { orb: "working", eyes: "working" },
  searching: { orb: "searching", eyes: "searching" },
  weaving: { orb: "weaving", eyes: "weaving" },
  composing: { orb: "composing", eyes: "composing" },
  listening: { orb: "listening", eyes: "listening" },
  idle: { orb: "breathing", eyes: "idle" },
};

type Approval = { id: number; title: string; detail: Record<string, unknown> };
type Answer = { ask: string; text: string; brief: Brief | null };

function useReducedMotion() {
  const [reduced, setReduced] = useState(() => matchMedia("(prefers-reduced-motion: reduce)").matches);
  useEffect(() => {
    const q = matchMedia("(prefers-reduced-motion: reduce)");
    const change = () => setReduced(q.matches);
    q.addEventListener("change", change);
    return () => q.removeEventListener("change", change);
  }, []);
  return reduced;
}

/** Drag the native panel by a handle; a press without movement stays a click. */
function useDrag() {
  const moved = useRef(false);
  const onPointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    moved.current = false;
    let last = { x: e.screenX, y: e.screenY };
    const start = { ...last };
    const move = (ev: PointerEvent) => {
      if (!moved.current && Math.hypot(ev.screenX - start.x, ev.screenY - start.y) < 4) return;
      moved.current = true;
      send({ type: "drag", dx: ev.screenX - last.x, dy: ev.screenY - last.y });
      last = { x: ev.screenX, y: ev.screenY };
    };
    const up = () => {
      removeEventListener("pointermove", move);
      removeEventListener("pointerup", up);
    };
    addEventListener("pointermove", move);
    addEventListener("pointerup", up);
  };
  /** Wraps a click handler so it doesn't fire at the end of a drag. */
  const click = (handler: () => void) => () => {
    if (!moved.current) handler();
    moved.current = false;
  };
  return { onPointerDown, click };
}

function App() {
  const drag = useDrag();
  const reduced = useReducedMotion();
  const [open, setOpen] = useState(false);
  const [text, setText] = useState("");
  const [asking, setAsking] = useState<string | null>(null);
  const [state, setState] = useState({ state: "idle", label: "Ready" });
  const [approval, setApproval] = useState<Approval | null>(null);
  const [answer, setAnswer] = useState<Answer | null>(null);
  const [error, setError] = useState<string | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const askRef = useRef<string | null>(null);
  const voice = useVoice((heard) => submit(heard));

  useEffect(() => {
    listen((m: Incoming) => {
      switch (m.type) {
        case "state":
          setState({ state: m.state, label: m.label });
          break;
        case "approval":
          setApproval({ id: m.id, title: m.title, detail: m.detail });
          break;
        case "answer":
          setAnswer({ ask: askRef.current ?? "", text: m.text, brief: m.brief });
          break;
        case "error":
          setError(m.message);
          break;
        case "voice":
        case "transcript":
        case "voice-error":
          voice.receive(m);
          break;
        case "done":
          setApproval(null);
          setAsking(null);
          askRef.current = null;
          break;
      }
    });
  }, []);

  // Report the content size so the native panel hugs it.
  useLayoutEffect(() => {
    const el = root.current;
    if (!el) return;
    const report = () => {
      const r = el.getBoundingClientRect();
      send({ type: "size", width: Math.ceil(r.width), height: Math.ceil(r.height) });
    };
    const observer = new ResizeObserver(report);
    observer.observe(el);
    report();
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    send({ type: "expanded", expanded: open });
    if (open) setTimeout(() => input.current?.focus(), 60);
  }, [open]);

  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (voice.listening) voice.cancel();
      else if (asking) send({ type: "cancel" });
      else setOpen(false);
    };
    addEventListener("keydown", key);
    return () => removeEventListener("keydown", key);
  }, [asking, voice.listening]);

  const submit = (spoken?: string) => {
    const ask = (spoken ?? text).trim();
    if (!ask || asking) return;
    askRef.current = ask;
    setAsking(ask);
    setAnswer(null);
    setError(null);
    setText("");
    setState({ state: "working", label: "Thinking" });
    send({ type: "ask", text: ask });
  };

  const decide = (approve: boolean) => {
    if (!approval) return;
    send({ type: "approve", id: approval.id, approve });
    setApproval(null);
  };

  // While an approval is open, Astrid is visibly waiting on you, whatever
  // late runtime states arrive.
  const look = approval ? looks.listening : looks[state.state] ?? looks.idle;
  const label = approval ? "Waiting for you" : state.label;
  const busy = asking !== null;

  if (!open) {
    return (
      <div className="pr-root pr-collapsed" ref={root}>
        <button
          type="button"
          className="pr-mark"
          aria-label="Ask Astrid (drag to move)"
          onPointerDown={drag.onPointerDown}
          onClick={drag.click(() => setOpen(true))}
        >
          <AstridLogo animated tracking />
        </button>
      </div>
    );
  }

  return (
    <div className="pr-root" ref={root}>
      <Liquid blur={8} contrast={20} fill="#151515" shadow="inset 0 0 0 1px #2a2a2a">
        {(voice.listening || voice.heard !== null || voice.error) && !busy && (
          <Liquid.Item transition="smooth">
            <ListeningCard voice={voice} reduced={reduced} />
          </Liquid.Item>
        )}
        {(busy || answer || error) && !voice.listening && (
          <Liquid.Item transition="smooth">
            <div className="pr-card" aria-live="polite">
              {busy && (
                <div className="pr-status pr-handle" onPointerDown={drag.onPointerDown}>
                  <AstridLogo expression={look.eyes} />
                  <ThinkingOrb state={look.orb} size={20} theme="dark" paused={reduced} />
                  <span>{label}</span>
                </div>
              )}
              {busy && <p className="pr-ask">“{asking}”</p>}
              {approval && <ApprovalCard approval={approval} onDecide={decide} />}
              {error && !busy && <p className="pr-error">{error}</p>}
              {answer && !busy && <AnswerView answer={answer} />}
            </div>
          </Liquid.Item>
        )}
        <div className="pr-input-row">
          <Liquid.Item transition="bouncy">
            <form
              className="pr-input"
              onSubmit={(e) => {
                e.preventDefault();
                submit();
              }}
            >
              <button
                type="button"
                className="pr-input-mark"
                aria-label="Close (drag to move)"
                onPointerDown={drag.onPointerDown}
                onClick={drag.click(() => setOpen(false))}
              >
                {busy ? <AstridLogo expression={look.eyes} /> : <AstridLogo animated tracking />}
              </button>
              <input
                ref={input}
                value={text}
                disabled={busy}
                placeholder={busy ? "Astrid is on it…" : "Ask about your email…"}
                onChange={(e) => setText(e.target.value)}
                aria-label="Ask Astrid"
              />
              {!busy && !text.trim() && (
                <button
                  type="button"
                  className={`pr-mic${voice.listening ? " pr-mic-on" : ""}`}
                  aria-label={voice.listening ? "Stop listening and send" : "Hold to talk"}
                  aria-pressed={voice.listening}
                  {...voice.handlers}
                >
                  <HugeiconsIcon icon={Mic01Icon} size={18} strokeWidth={2} />
                </button>
              )}
            </form>
          </Liquid.Item>
          {/* The send button lives behind the input's right edge and springs out
              through the goo as soon as there is something to send. */}
          <Liquid.Item
            transition="bouncy"
            x={text.trim() && !busy ? 0 : -(44 + 24)}
            scale={text.trim() && !busy ? 1 : 0.3}
          >
            <button
              type="button"
              className={`pr-send${text.trim() && !busy ? "" : " pr-send-hidden"}`}
              aria-label="Send"
              onClick={() => submit()}
              tabIndex={text.trim() && !busy ? 0 : -1}
            >
              <HugeiconsIcon icon={ArrowUp02Icon} size={20} strokeWidth={2} />
            </button>
          </Liquid.Item>
        </div>
      </Liquid>
    </div>
  );
}

type Voice = ReturnType<typeof useVoice>;

/**
 * Push-to-talk (V2-03, ADR 0034). Hold the mic to talk and let go to send; a
 * quick tap keeps listening until the next tap. The native host owns the mic
 * and on-device speech and streams back the level and transcript. In a plain
 * browser the ring follows the page's own microphone instead.
 */
function useVoice(onHeard: (text: string) => void) {
  const [listening, setListening] = useState(false);
  const [heard, setHeard] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const level = useRef(0);
  const pressed = useRef(0);
  const cancelled = useRef(false);
  const heardRef = useRef(onHeard);
  heardRef.current = onHeard;
  const mic = useMicrophone();

  const start = () => {
    if (listening) return;
    cancelled.current = false;
    level.current = 0;
    setHeard("");
    setError(null);
    setListening(true);
    if (!native() && mic.supported) mic.start();
    send({ type: "listen", on: true });
  };
  const stop = () => {
    if (!listening) return;
    setListening(false);
    if (!native()) mic.stop();
    send({ type: "listen", on: false });
  };
  const cancel = () => {
    cancelled.current = true;
    stop();
    setHeard(null);
  };

  const receive = (m: Incoming) => {
    if (m.type === "voice") level.current = m.level;
    else if (m.type === "voice-error") {
      setListening(false);
      setHeard(null);
      setError(m.message);
    } else if (m.type === "transcript") {
      if (cancelled.current) return;
      setHeard(m.text);
      if (m.final) {
        setListening(false);
        setHeard(null);
        if (m.text.trim()) heardRef.current(m.text);
      }
    }
  };

  const handlers = {
    onPointerDown: (e: React.PointerEvent) => {
      if (e.button !== 0) return;
      e.preventDefault();
      if (listening) return stop();
      pressed.current = performance.now();
      start();
    },
    onPointerUp: () => {
      // A hold sends on release; a tap leaves the mic open until the next tap.
      if (pressed.current && performance.now() - pressed.current > 350) stop();
      pressed.current = 0;
    },
    onKeyDown: (e: React.KeyboardEvent) => {
      if (e.key !== " " && e.key !== "Enter") return;
      e.preventDefault();
      if (!e.repeat) (listening ? stop : start)();
    },
  };

  return { listening, heard, error, level, stream: native() ? null : mic.stream, start, stop, cancel, receive, handlers };
}

function ListeningCard({ voice, reduced }: { voice: Voice; reduced: boolean }) {
  return (
    <div className="pr-listen">
      <div className="pr-listen-ring">
        <VoiceRing
          size={132}
          theme="dark"
          state={voice.listening ? "listening" : voice.error ? "idle" : "processing"}
          stream={voice.stream}
          level={voice.stream ? undefined : () => voice.level.current}
          paused={reduced}
        />
        <span className="pr-listen-mark">
          <AstridLogo expression="listening" />
        </span>
      </div>
      {voice.error ? (
        <p className="pr-error">{voice.error}</p>
      ) : (
        <p className={`pr-heard${voice.heard ? "" : " pr-heard-empty"}`} aria-live="polite">
          {voice.heard || (voice.listening ? "Listening…" : "")}
        </p>
      )}
      {!voice.error && <span className="pr-eyebrow">ON-DEVICE · NOTHING SAVED</span>}
    </div>
  );
}

function ApprovalCard({ approval, onDecide }: { approval: Approval; onDecide: (ok: boolean) => void }) {
  return (
    <div className="pr-approval">
      <span className="pr-eyebrow">ONE-TIME PERMISSION · READ-ONLY</span>
      <p>{approval.title}</p>
      <small>Results are shared with the model for this answer only. Astrid can’t send, delete or change mail.</small>
      <div className="pr-approval-actions">
        <button type="button" className="pr-btn pr-btn-ghost" onClick={() => onDecide(false)}>
          Don’t
        </button>
        <button type="button" className="pr-btn" onClick={() => onDecide(true)}>
          Allow once
        </button>
      </div>
    </div>
  );
}

function AnswerView({ answer }: { answer: Answer }) {
  const { brief } = answer;
  const sources = new Map((brief?.sources ?? []).map((s) => [s.id, s]));
  const [done, setDone] = useState<Set<number>>(new Set());
  if (!brief) {
    return (
      <div className="pr-brief">
        <p className="pr-asked">{answer.ask}</p>
        <Prose text={answer.text} />
      </div>
    );
  }
  return (
    <div className="pr-brief">
      <header>
        <span className="pr-eyebrow">BRIEF · {new Date().toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}</span>
        {brief.title && <h1>{brief.title}</h1>}
        {brief.summary && <p className="pr-caption">{brief.summary}</p>}
      </header>
      {!!brief.todos?.length && (
        <section>
          <h2>To-dos</h2>
          <ul className="pr-todos">
            {brief.todos.map((t, i) => (
              <li key={i} className={done.has(i) ? "done" : ""}>
                <button
                  type="button"
                  className="pr-check"
                  aria-label={done.has(i) ? "Mark not done" : "Mark done"}
                  onClick={() => setDone((d) => new Set(d.has(i) ? [...d].filter((x) => x !== i) : [...d, i]))}
                />
                <span>{t.text}</span>
                {t.source != null && <SourceBadge id={t.source} source={sources.get(t.source)} />}
              </li>
            ))}
          </ul>
        </section>
      )}
      {!!brief.updates?.length && (
        <section>
          <h2>Updates</h2>
          <ol className="pr-updates">
            {brief.updates.map((u, i) => (
              <li key={i}>
                <b>{String(i + 1).padStart(2, "0")}</b>
                <span>{u.text}</span>
                {u.source != null && <SourceBadge id={u.source} source={sources.get(u.source)} />}
              </li>
            ))}
          </ol>
        </section>
      )}
      {!!brief.sources?.length && (
        <footer className="pr-sources">
          {brief.sources.map((s) => (
            <SourceBadge key={s.id} id={s.id} source={s} label />
          ))}
        </footer>
      )}
    </div>
  );
}

type Source = NonNullable<Brief["sources"]>[number];

function SourceBadge({ id, source, label }: { id: number; source?: Source; label?: boolean }) {
  const url =
    source?.kind === "gmail" && source.message_id && /^[0-9a-f]+$/i.test(source.message_id)
      ? `https://mail.google.com/mail/u/0/#all/${source.message_id}`
      : null;
  return (
    <span className={`pr-badge${label ? " pr-badge-label" : ""}`}>
      <button type="button" onClick={() => url && send({ type: "open", url })} disabled={!url} aria-label={`Source ${id}`}>
        <HugeiconsIcon icon={Mail01Icon} size={12} strokeWidth={2} />
        <span>{label ? source?.from ?? `Source ${id}` : id}</span>
      </button>
      {source && (
        <span className="pr-pop" role="tooltip">
          <b>{source.subject ?? "Email"}</b>
          <span>
            {source.from}
            {source.date ? ` · ${source.date}` : ""}
          </span>
          {url && <i>Open in Gmail</i>}
        </span>
      )}
    </span>
  );
}

/** Minimal formatting for plain answers: paragraphs, lists and **bold**. */
function Prose({ text }: { text: string }) {
  const inline = (line: string): ReactNode[] =>
    line.split(/(\*\*[^*]+\*\*)/g).map((part, i) =>
      part.startsWith("**") && part.endsWith("**") ? <b key={i}>{part.slice(2, -2)}</b> : part,
    );
  return (
    <div className="pr-prose">
      {text.split(/\n{2,}/).map((block, i) => {
        const lines = block.split("\n");
        if (lines.every((l) => /^\s*(\d+\.|[-*])\s/.test(l))) {
          return (
            <ul key={i}>
              {lines.map((l, j) => (
                <li key={j}>{inline(l.replace(/^\s*(\d+\.|[-*])\s/, ""))}</li>
              ))}
            </ul>
          );
        }
        return <p key={i}>{inline(block)}</p>;
      })}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
