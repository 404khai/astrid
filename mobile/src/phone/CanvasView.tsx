import { useEffect, useRef } from "react";
import { StyleSheet, View, type ViewStyle } from "react-native";
import { WebView, type WebViewMessageEvent } from "react-native-webview";
import type { Shape } from "./store";

/**
 * Astrid's live canvas on the phone (ADR 0039): the same page as presence's
 * canvas panel (`web/canvas.html`, ADR 0036), built into one file in the APK
 * and fed the runtime's validated shapes. A live reply streams in shape by
 * shape with the cursor tour; a saved one is replayed at once. Drawing is
 * presentation only; the page reads nothing and loads nothing else.
 */
const PAGE = "file:///android_asset/astrid-canvas.html?embed";

// The page talks to its host through WKWebView's bridge; this is that bridge.
const BRIDGE = `
  window.webkit = { messageHandlers: { astrid: { postMessage: function (m) {
    window.ReactNativeWebView.postMessage(JSON.stringify(m));
  } } } };
  document.documentElement.classList.add("cv-phone");
  true;
`;

export function CanvasView({
  shapes,
  animate,
  finished,
  style,
  interactive = true,
}: {
  shapes: Shape[];
  animate: boolean;
  finished: boolean;
  style?: ViewStyle;
  /** False for a thumbnail in a scrolling thread: touches go to the thread. */
  interactive?: boolean;
}) {
  const web = useRef<WebView>(null);
  const feed = useRef({ ready: false, sent: 0, begun: false, ended: false, animate });

  const post = (message: unknown) => {
    web.current?.injectJavaScript(`window.astrid && window.astrid.receive(${JSON.stringify(message)}); true;`);
  };

  const sync = () => {
    const f = feed.current;
    if (!f.ready) return;
    if (!f.animate) {
      if (shapes.length === f.sent) return;
      post({ type: "canvas", event: "replay", shapes });
      f.sent = shapes.length;
      return;
    }
    if (!f.begun) {
      post({ type: "canvas", event: "begin", width: 1000, height: 640 });
      f.begun = true;
    }
    while (f.sent < shapes.length) post({ type: "canvas", event: "shape", shape: shapes[f.sent++] });
    if (finished && !f.ended) {
      post({ type: "canvas", event: "end" });
      f.ended = true;
    }
  };

  useEffect(sync, [shapes, finished]);

  const onMessage = (event: WebViewMessageEvent) => {
    try {
      if (JSON.parse(event.nativeEvent.data).type === "ready") {
        feed.current.ready = true;
        sync();
      }
    } catch {}
  };

  return (
    <View style={[styles.frame, style]} pointerEvents={interactive ? "auto" : "none"}>
      <WebView
        ref={web}
        source={{ uri: PAGE }}
        originWhitelist={["file://*"]}
        allowFileAccess
        injectedJavaScriptBeforeContentLoaded={BRIDGE}
        onMessage={onMessage}
        onShouldStartLoadWithRequest={(request) => request.url.startsWith("file:///android_asset/astrid-canvas.html")}
        style={styles.web}
        containerStyle={styles.web}
        overScrollMode="never"
        scrollEnabled={false}
        setSupportMultipleWindows={false}
        javaScriptCanOpenWindowsAutomatically={false}
        cacheEnabled={false}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  frame: { overflow: "hidden", borderRadius: 18, backgroundColor: "#141418" },
  web: { flex: 1, backgroundColor: "transparent" },
});
