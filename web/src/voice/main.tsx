import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { VoiceRing } from "voice-ring";
import { AstridLogo } from "../components/AstridLogo";
import "./voice.css";

/**
 * The desktop app's voice mode orb (ADR 0037 amendment): presence's voice ring
 * round Astrid's mark, fed the Mac's mic or speaker frames over the WKWebView
 * bridge exactly as presence's is (ADR 0034). Tapping her interrupts.
 */
type State = "listening" | "speaking" | "processing" | "idle";
type Incoming =
  | { type: "state"; state: State }
  | { type: "voice"; sampleRate: number; time: string; freq: string }
  | { type: "reduced"; on: boolean };

const host = window as unknown as {
  astrid?: { receive: (m: Incoming) => void };
  webkit?: { messageHandlers?: { astrid?: { postMessage: (m: unknown) => void } } };
};

/** A Web Audio AnalyserNode's face over the host's frames (as presence's). */
class HostAnalyser {
  fftSize = 1024;
  frequencyBinCount = 512;
  context = { sampleRate: 48000 };
  private time = new Float32Array(1024);
  private freq = new Uint8Array(512);

  write(sampleRate: number, time: string, freq: string) {
    this.context.sampleRate = sampleRate;
    const t = Uint8Array.from(atob(time), (c) => c.charCodeAt(0));
    this.time.set(new Float32Array(t.buffer, 0, Math.min(1024, t.byteLength >> 2)));
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

const analyser = new HostAnalyser();

function Orb() {
  const [state, setState] = useState<State>("idle");
  const [reduced, setReduced] = useState(() => matchMedia("(prefers-reduced-motion: reduce)").matches);

  useEffect(() => {
    host.astrid = {
      receive: (m) => {
        if (m.type === "voice") analyser.write(m.sampleRate, m.time, m.freq);
        else if (m.type === "state") {
          analyser.clear();
          setState(m.state);
        } else if (m.type === "reduced") setReduced(m.on);
      },
    };
    host.webkit?.messageHandlers?.astrid?.postMessage({ type: "ready" });
  }, []);

  const expression = state === "speaking" ? "composing" : state === "processing" ? "solving" : state === "idle" ? "idle" : "listening";
  return (
    <div className="vo-ring">
      <VoiceRing size={200} theme="dark" state={state} analyser={analyser as unknown as AnalyserNode} paused={reduced} />
      <button
        type="button"
        className="vo-mark"
        aria-label="Interrupt Astrid"
        onClick={() => host.webkit?.messageHandlers?.astrid?.postMessage({ type: "interrupt" })}
      >
        <AstridLogo expression={expression} />
      </button>
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Orb />
  </StrictMode>,
);
