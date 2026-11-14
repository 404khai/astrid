import { useEffect, useRef } from "react";
import { StyleSheet, View, type ViewStyle } from "react-native";
import { WebView, type WebViewMessageEvent } from "react-native-webview";
import { toBase64 } from "../link/crypto";
import { builtInBlobs } from "./blobs.generated";

/**
 * Pages from `web/` built into the APK (scripts/build-canvas.mjs): presence's
 * voice ring and the desktop's blobs row. They talk to their host through the
 * WKWebView-style bridge the pages expect, shimmed here. They load nothing
 * else and navigation away is refused.
 */
const BRIDGE = `
  window.webkit = { messageHandlers: { astrid: { postMessage: function (m) {
    window.ReactNativeWebView.postMessage(JSON.stringify(m));
  } } } };
  true;
`;

function useAssetPage(page: string, onMessage: (message: Record<string, unknown>) => void) {
  const web = useRef<WebView>(null);
  const ready = useRef(false);
  const queue = useRef<unknown[]>([]);
  const post = (message: unknown) => {
    if (!ready.current) {
      queue.current.push(message);
      return;
    }
    web.current?.injectJavaScript(`window.astrid && window.astrid.receive(${JSON.stringify(message)}); true;`);
  };
  const view = (style?: ViewStyle) => (
    <WebView
      ref={web}
      source={{ uri: `file:///android_asset/astrid-${page}.html` }}
      originWhitelist={["file://*"]}
      allowFileAccess
      injectedJavaScriptBeforeContentLoaded={BRIDGE}
      onMessage={(event: WebViewMessageEvent) => {
        try {
          const message = JSON.parse(event.nativeEvent.data);
          if (message.type === "ready") {
            ready.current = true;
            const pending = queue.current;
            queue.current = [];
            pending.forEach(post);
          }
          onMessage(message);
        } catch {}
      }}
      onShouldStartLoadWithRequest={(request) => request.url.startsWith(`file:///android_asset/astrid-${page}.html`)}
      style={[styles.web, style]}
      containerStyle={styles.web}
      scrollEnabled={false}
      overScrollMode="never"
      setSupportMultipleWindows={false}
      cacheEnabled={false}
    />
  );
  return { post, view };
}

export type RingState = "listening" | "speaking" | "processing" | "idle";

/** Presence's voice ring round Astrid's mark. Android gives the mic's loudness,
 * not its samples, so the ring is fed a tone shaped by that loudness. */
export function VoiceRing({ state, level, size, onTap }: { state: RingState; level: number; size: number; onTap: () => void }) {
  const { post, view } = useAssetPage("voice", (m) => m.type === "interrupt" && onTap());
  useEffect(() => post({ type: "state", state }), [state]);
  const phase = useRef(0);
  useEffect(() => {
    if (state === "idle" || state === "processing") return;
    const time = new Float32Array(1024);
    const freq = new Uint8Array(512);
    const timer = setInterval(() => {
      const loud = state === "speaking" ? 0.35 + 0.25 * Math.random() : level;
      for (let i = 0; i < time.length; i++) {
        phase.current += 0.045 + 0.02 * Math.sin(i / 90);
        time[i] = loud * 0.7 * Math.sin(phase.current) * (0.8 + 0.2 * Math.random());
      }
      for (let i = 0; i < freq.length; i++) {
        const band = Math.exp(-i / (40 + 80 * loud));
        freq[i] = Math.min(255, Math.round(255 * loud * band * (0.75 + 0.25 * Math.random())));
      }
      post({
        type: "voice",
        sampleRate: 48000,
        time: toBase64(new Uint8Array(time.buffer)),
        freq: toBase64(freq),
      });
    }, 50);
    return () => clearInterval(timer);
  }, [state, level]);
  return <View style={{ width: size, height: size }}>{view()}</View>;
}

export type BlobItem = { id: string; name: string; character: unknown };
export const allBlobs: BlobItem[] = builtInBlobs as unknown as BlobItem[];
export const defaultBlobs = ["06-sunee.avatar.json", "08-cloudee.avatar.json", "03-citrus.avatar.json", "02-freddy.avatar.json", "07-kirby.avatar.json"];

/** The row of blobs, as on the desktop's sidebar. */
export function BlobsRow({ shown, selected, busy, onPick }: { shown: string[]; selected: string | null; busy: boolean; onPick: (id: string) => void }) {
  const items = shown.map((id) => allBlobs.find((b) => b.id === id)).filter(Boolean).slice(0, 5) as BlobItem[];
  const { post, view } = useAssetPage("blobs", (m) => m.type === "select" && typeof m.id === "string" && onPick(m.id));
  useEffect(() => post({ type: "blobs", items, selected: selected ?? items[0]?.id ?? null }), [shown.join(",")]);
  useEffect(() => post({ type: "state", state: busy ? "thinking" : "idle" }), [busy]);
  if (!items.length) return null;
  return <View style={{ height: 50 }}>{view()}</View>;
}

const styles = StyleSheet.create({
  web: { flex: 1, backgroundColor: "transparent" },
});
