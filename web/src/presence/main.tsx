import { StrictMode, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource/space-grotesk/latin-400.css";
import "@fontsource/space-grotesk/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import { Liquid } from "liquid-gooey";
import { ThinkingOrb } from "thinking-orbs";
import type { OrbState } from "thinking-orbs";
import { HugeiconsIcon } from "@hugeicons/react";
import { ArrowUp02Icon, Mail01Icon, Mic01Icon, StopIcon, VolumeHighIcon } from "@hugeicons/core-free-icons";
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

/** What Astrid says out loud: the gist, not every to-do and source (ADR 0035). */
function spoken(answer: Answer) {
  const plain = (t: string) => t.replace(/\*\*|__|`|#+\s|^\s*[-*]\s|^\s*\d+\.\s/gm, "").replace(/\s+/g, " ").trim();
  const end = (t: string) => (/[.!?…]$/.test(t) ? t : `${t}.`);
  if (answer.brief?.title || answer.brief?.summary) {
    return [answer.brief.title, answer.brief.summary].filter(Boolean).map((t) => end(plain(t!))).join(" ");
  }
  const text = plain(answer.text);
  if (text.length <= 600) return text;
  const cut = text.slice(0, 600);
  return cut.slice(0, Math.max(cut.lastIndexOf(". ") + 1, 300));
}

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
      if (!moved.current && native()) {
        // The host follows the mouse itself until release: no per-move round trip.
        moved.current = true;
        send({ type: "drag-start" });
        return;
      }
      moved.current = true;
      if (native()) return;
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
  const [speaking, setSpeaking] = useState(false);
  const speakerAnalyser = useRef(new HostAnalyser());
  // Answers to a dictated ask are read aloud; typed asks stay quiet unless
  // you press the speaker on the answer.
  const dictated = useRef(false);
  const askedByVoice = useRef(false);
  const voice = useVoice((heard, final) => {
    setText(heard);
    if (final) {
      dictated.current = heard.trim().length > 0;
      setTimeout(() => input.current?.focus(), 0);
    }
  });
  const voiceRef = useRef(voice);
  voiceRef.current = voice;

  useEffect(() => {
    listen((m: Incoming) => {
      switch (m.type) {
        case "state":
          setState({ state: m.state, label: m.label });
          break;
        case "approval":
          setApproval({ id: m.id, title: m.title, detail: m.detail });
          break;
        case "answer": {
          const next = { ask: askRef.current ?? "", text: m.text, brief: m.brief };
          setAnswer(next);
          if (askedByVoice.current) send({ type: "speak", text: spoken(next) });
          break;
        }
        case "speaking":
          setSpeaking(m.on);
          if (!m.on) speakerAnalyser.current.clear();
          break;
        case "error":
          setError(m.message);
          break;
        case "hotkey":
          // Hold-to-talk from anywhere: open the panel and listen until release.
          if (m.down) {
            setOpen(true);
            voiceRef.current.start();
          } else voiceRef.current.stop();
          break;
        case "voice":
          if (m.source === "speaker") {
            speakerAnalyser.current.write(m.sampleRate, m.time, m.freq);
            break;
          }
          voiceRef.current.receive(m);
          break;
        case "transcript":
        case "voice-error":
          voiceRef.current.receive(m);
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
      else if (speaking) send({ type: "speak-stop" });
      else if (asking) send({ type: "cancel" });
      else setOpen(false);
    };
    addEventListener("keydown", key);
    return () => removeEventListener("keydown", key);
  }, [asking, voice.listening, speaking]);

  const submit = (said?: string) => {
    const ask = (said ?? text).trim();
    if (!ask || asking) return;
    askRef.current = ask;
    askedByVoice.current = said !== undefined || dictated.current;
    dictated.current = false;
    if (speaking) send({ type: "speak-stop" });
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
      {voice.listening && !busy ? (
        <VoiceHalo state="listening" stream={voice.stream} analyser={voice.analyser.current} reduced={reduced} />
      ) : speaking ? (
        <VoiceHalo state="speaking" stream={null} analyser={speakerAnalyser.current} reduced={reduced} />
      ) : null}
      <Liquid blur={8} contrast={20} fill="#151515" shadow="inset 0 0 0 1px #2a2a2a">
        {voice.error && !busy && (
          <Liquid.Item transition="smooth">
            <div className="pr-card">
              <p className="pr-error">{voice.error}</p>
            </div>
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
              {answer && !busy && (
                <AnswerView
                  answer={answer}
                  speaking={speaking}
                  onSpeak={() => send(speaking ? { type: "speak-stop" } : { type: "speak", text: spoken(answer) })}
                />
              )}
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
                placeholder={busy ? "Astrid is on it…" : voice.listening ? "Listening…" : "Ask about your email…"}
                onChange={(e) => {
                  setText(e.target.value);
                  if (!e.target.value.trim()) dictated.current = false;
                }}
                aria-label="Ask Astrid"
              />
              {!busy && (!text.trim() || voice.listening) && (
                <button
                  type="button"
                  className={`pr-mic${voice.listening ? " pr-mic-on" : ""}`}
                  aria-label={voice.listening ? "Stop listening and send" : "Hold to talk (or hold ⌥Space anywhere)"}
                  title="Hold to talk · ⌥Space"
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
            x={text.trim() && !busy && !voice.listening ? 0 : -(44 + 24)}
            scale={text.trim() && !busy && !voice.listening ? 1 : 0.3}
          >
            <button
              type="button"
              className={`pr-send${text.trim() && !busy && !voice.listening ? "" : " pr-send-hidden"}`}
              aria-label="Send"
              onClick={() => submit()}
              tabIndex={text.trim() && !busy && !voice.listening ? 0 : -1}
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
 * Push-to-talk (V2-03, ADR 0034). Hold the mic (or ⌥Space) to talk; a quick
 * tap keeps listening until the next tap. What you say is written into the
 * input as you speak, ready to edit and send. The native host owns the mic and
 * on-device speech and streams back analyser frames and the transcript. In a
 * plain browser the ring follows the page's own microphone instead.
 */
function useVoice(onHeard: (text: string, final: boolean) => void) {
  const [listening, setListening] = useState(false);
  const [heard, setHeard] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const analyser = useRef(new HostAnalyser());
  const on = useRef(false);
  const pressed = useRef(0);
  const cancelled = useRef(false);
  const heardRef = useRef(onHeard);
  heardRef.current = onHeard;
  const mic = useMicrophone();

  const start = () => {
    if (on.current) return;
    on.current = true;
    cancelled.current = false;
    analyser.current.clear();
    setHeard("");
    setError(null);
    setListening(true);
    if (!native() && mic.supported) mic.start();
    send({ type: "listen", on: true });
  };
  const stop = () => {
    if (!on.current) return;
    on.current = false;
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
    if (m.type === "voice") analyser.current.write(m.sampleRate, m.time, m.freq);
    else if (m.type === "voice-error") {
      on.current = false;
      setListening(false);
      setHeard(null);
      setError(m.message);
    } else if (m.type === "transcript") {
      if (cancelled.current) return;
      setHeard(m.text);
      heardRef.current(m.text, m.final);
      if (m.final) {
        on.current = false;
        setListening(false);
        setHeard(null);
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

  return { listening, heard, error, analyser, stream: native() ? null : mic.stream, start, stop, cancel, receive, handlers };
}

/**
 * Stands in for a Web Audio AnalyserNode, filled from the host's frames, so the
 * voice ring reads the native microphone through its full-spectrum path
 * (loudness, voice bands and spectrum) exactly as it reads a browser mic.
 */
class HostAnalyser {
  fftSize = 1024;
  frequencyBinCount = 512;
  context = { sampleRate: 48000 };
  private time = new Float32Array(1024);
  private freq = new Uint8Array(512);

  write(sampleRate: number, time: string, freq: string) {
    this.context.sampleRate = sampleRate;
    const t = Uint8Array.from(atob(time), (c) => c.charCodeAt(0));
    const samples = new Float32Array(t.buffer, 0, Math.min(1024, t.byteLength >> 2));
    this.time.set(samples);
    const f = atob(freq);
    for (let i = 0; i < 512 && i < f.length; i++) this.freq[i] = f.charCodeAt(i);
  }
  clear() {
    this.time.fill(0);
    this.freq.fill(0);
  }
  getFloatTimeDomainData(out: Float32Array) {
    out.set(this.time.subarray(0, out.length));
  }
  getByteFrequencyData(out: Uint8Array) {
    out.set(this.freq.subarray(0, out.length));
  }
}

/**
 * The voice ring floats above the input on the bare desktop, no card behind
 * it: following your mic while you talk, and Astrid's own voice while she speaks.
 */
function VoiceHalo({
  state,
  stream,
  analyser,
  reduced,
}: {
  state: "listening" | "speaking";
  stream: MediaStream | null;
  analyser: HostAnalyser;
  reduced: boolean;
}) {
  return (
    <div className="pr-listen-ring" aria-hidden>
      <VoiceRing
        size={200}
        theme="dark"
        state={state}
        stream={stream}
        analyser={stream ? undefined : (analyser as unknown as AnalyserNode)}
        paused={reduced}
      />
      <span className="pr-listen-mark">
        <AstridLogo expression={state === "listening" ? "listening" : "composing"} />
      </span>
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

function SpeakButton({ speaking, onSpeak }: { speaking: boolean; onSpeak: () => void }) {
  return (
    <button
      type="button"
      className={`pr-speak${speaking ? " pr-speak-on" : ""}`}
      aria-label={speaking ? "Stop speaking" : "Read aloud"}
      title={speaking ? "Stop · Esc" : "Read aloud"}
      onClick={onSpeak}
    >
      <HugeiconsIcon icon={speaking ? StopIcon : VolumeHighIcon} size={14} strokeWidth={2} />
    </button>
  );
}

function AnswerView({ answer, speaking, onSpeak }: { answer: Answer; speaking: boolean; onSpeak: () => void }) {
  const { brief } = answer;
  const sources = new Map((brief?.sources ?? []).map((s) => [s.id, s]));
  const [done, setDone] = useState<Set<number>>(new Set());
  if (!brief) {
    return (
      <div className="pr-brief">
        <div className="pr-answer-head">
          <p className="pr-asked">{answer.ask}</p>
          <SpeakButton speaking={speaking} onSpeak={onSpeak} />
        </div>
        <Prose text={answer.text} />
      </div>
    );
  }
  return (
    <div className="pr-brief">
      <header>
        <div className="pr-answer-head">
          <span className="pr-eyebrow">BRIEF · {new Date().toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}</span>
          <SpeakButton speaking={speaking} onSpeak={onSpeak} />
        </div>
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
