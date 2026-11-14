/** Messages between the ask surface and the native presence app (ADR 0033). */
export type Brief = {
  title?: string;
  summary?: string;
  todos?: { text: string; source?: number }[];
  updates?: { text: string; source?: number }[];
  sources?: { id: number; kind?: string; from?: string; subject?: string; date?: string; message_id?: string }[];
};

export type Incoming =
  | { type: "ready"; model?: string }
  | { type: "state"; state: string; label: string }
  | { type: "delta"; text: string }
  | { type: "approval"; id: number; title: string; detail: Record<string, unknown> }
  | { type: "answer"; text: string; brief: Brief | null; canvas?: boolean }
  | { type: "error"; message: string }
  | { type: "done"; outcome: string }
  | { type: "voice"; sampleRate: number; time: string; freq: string; source?: "speaker" }
  | { type: "speaking"; on: boolean }
  | { type: "hotkey"; down: boolean }
  | { type: "transcript"; text: string; final: boolean }
  | { type: "voice-error"; message: string }
  | { type: "canvas-open"; on: boolean }
  // ADR 0038 operator: a copy of the step card on screen, and what voice matched.
  | { type: "operator"; on: boolean }
  | {
      type: "operator-card";
      id: number;
      text: string;
      step: number | null;
      total: number | null;
      buttons: { id: string; label: string }[];
    }
  | { type: "operator-card-clear"; id: number }
  | { type: "heard"; text: string; choice: string | null };

export type Outgoing =
  | { type: "ask"; text: string }
  | { type: "utterance"; text: string }
  | { type: "choice"; id: number; choice: string }
  | { type: "approve"; id: number; approve: boolean }
  | { type: "cancel" }
  | { type: "listen"; on: boolean }
  | { type: "speak"; text: string }
  | { type: "speak-stop" }
  | { type: "open"; url: string }
  | { type: "size"; width: number; height: number }
  | { type: "drag"; dx: number; dy: number }
  | { type: "drag-start" }
  | { type: "expanded"; expanded: boolean };

declare global {
  interface Window {
    webkit?: { messageHandlers?: { astrid?: { postMessage(message: Outgoing): void } } };
    astrid?: { receive(message: Incoming): void };
  }
}

export const native = () => window.webkit?.messageHandlers?.astrid;
let listener: (message: Incoming) => void = () => {};

/** The native app calls window.astrid.receive(...) for every runtime event,
 * and with {type:"pointer"} for the global cursor in page coordinates. */
window.astrid = {
  receive: (message) => {
    if ((message as { type: string }).type === "pointer") return followPointer(message as unknown as { x: number; y: number });
    listener(message);
  },
};

/* AstridLogo tracks window pointermove events; replaying the system cursor as
 * one lets the mark follow the pointer anywhere on screen. After 4 s at rest
 * the eyes go back to their look-around loop. */
let restTimer: number | undefined;
function followPointer({ x, y }: { x: number; y: number }) {
  if (!Number.isFinite(x) || !Number.isFinite(y)) return;
  window.dispatchEvent(new PointerEvent("pointermove", { clientX: x, clientY: y, pointerType: "mouse" }));
  window.clearTimeout(restTimer);
  restTimer = window.setTimeout(() => document.dispatchEvent(new PointerEvent("pointerleave")), 4000);
}

export function listen(handler: (message: Incoming) => void) {
  listener = handler;
}

export function send(message: Outgoing) {
  const bridge = native();
  if (bridge) bridge.postMessage(message);
  else if (message.type === "ask") mock(message.text);
  else if (message.type === "approve") pendingMock?.(message.approve);
  else if (message.type === "listen") mockListen(message.on);
  else if (message.type === "speak") mockSpeak(message.text);
  else if (message.type === "speak-stop") speechSynthesis.cancel();
}

/** In a plain browser (vite dev), a scripted run so the UI can be designed. */
let pendingMock: ((ok: boolean) => void) | null = null;
function mock(text: string) {
  const say = (m: Incoming, ms: number) => new Promise<void>((r) => setTimeout(() => (listener(m), r()), ms));
  (async () => {
    await say({ type: "state", state: "working", label: "Thinking" }, 300);
    await say({ type: "approval", id: 1, title: `Search Gmail for “newer_than:2d”?`, detail: {} }, 900);
    // Real runtimes deliver these after the approval; the card must survive them.
    await say({ type: "state", state: "searching", label: "Searching mail" }, 30);
    await say({ type: "state", state: "listening", label: "Waiting for you" }, 30);
    const ok = await new Promise<boolean>((r) => (pendingMock = r));
    if (!ok) return say({ type: "error", message: "mail access was not approved" }, 200);
    await say({ type: "state", state: "searching", label: "Searching mail" }, 200);
    await say({ type: "state", state: "weaving", label: "Reading an email" }, 1400);
    await say({ type: "state", state: "composing", label: "Writing" }, 1400);
    await say(
      {
        type: "answer",
        text: `You asked: ${text}`,
        brief: {
          title: "Two replies need you today",
          summary: "A flight change and a design review are waiting; the rest is receipts.",
          todos: [
            { text: "Confirm the new 09:40 departure with Elodie", source: 1 },
            { text: "Leave comments on the onboarding review before Friday", source: 2 },
          ],
          updates: [
            { text: "Your Lagos flight moved 25 minutes earlier; seat 14A kept.", source: 1 },
            { text: "Stripe sent the October receipt for $24.00.", source: 3 },
          ],
          sources: [
            { id: 1, kind: "gmail", from: "Élodie Martin", subject: "Flight to Lagos confirmed", date: "Nov 12" },
            { id: 2, kind: "gmail", from: "Ada Okafor", subject: "Onboarding review", date: "Nov 12" },
            { id: 3, kind: "gmail", from: "Stripe", subject: "Your receipt", date: "Nov 11" },
          ],
        },
      },
      200,
    );
    await say({ type: "done", outcome: "completed" }, 50);
    await say({ type: "state", state: "idle", label: "Ready" }, 50);
  })();
}

/** Dev stand-in for on-device speech: the ring follows the browser mic, and a
 * scripted transcript types itself out while listening. */
let mockWords: number | undefined;
function mockListen(on: boolean) {
  const words = "what came in from Elodie today".split(" ");
  window.clearInterval(mockWords);
  if (on) {
    let n = 0;
    mockWords = window.setInterval(() => {
      n = Math.min(n + 1, words.length);
      listener({ type: "transcript", text: words.slice(0, n).join(" "), final: false });
    }, 350);
  } else {
    setTimeout(() => listener({ type: "transcript", text: words.join(" "), final: true }), 250);
  }
}

// Dev: hold ⌥Space in the page to stand in for the native global hotkey.
if (!native()) {
  let held = false;
  addEventListener("keydown", (e) => {
    if (e.code !== "Space" || !e.altKey || held) return;
    e.preventDefault();
    held = true;
    listener({ type: "hotkey", down: true });
  });
  addEventListener("keyup", (e) => {
    if (e.code !== "Space" || !held) return;
    held = false;
    listener({ type: "hotkey", down: false });
  });
}

/** Dev stand-in for astrid-voice: the browser's own speech, no ring data. */
function mockSpeak(text: string) {
  speechSynthesis.cancel();
  const utterance = new SpeechSynthesisUtterance(text);
  utterance.onstart = () => listener({ type: "speaking", on: true });
  utterance.onend = utterance.onerror = () => listener({ type: "speaking", on: false });
  speechSynthesis.speak(utterance);
}
