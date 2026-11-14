import { useEffect, useRef, useState } from "react";
import { Animated, Easing, Image, Pressable, StyleSheet, Text, TextInput, View } from "react-native";
import { CanvasView } from "./CanvasView";
import { visible, type Live, type Message } from "./store";
import type { LinkStatus } from "../link/client";

export const colors = {
  bg: "#0b0b10",
  card: "#15151c",
  line: "#26262f",
  text: "#f1f1f6",
  dim: "#9696a3",
  faint: "#6b6b78",
  blue: "#4459f9",
  deep: "#1a32ec",
  teal: "#00f7d5",
  red: "#f94447",
};

const logo = require("../../assets/logo.png");

export function Mark({ size = 28 }: { size?: number }) {
  return <Image source={logo} style={{ width: size, height: size * 0.8, resizeMode: "contain" }} accessibilityIgnoresInvertColors />;
}

/** A dot that breathes while Astrid is working. */
export function Pulse({ color = colors.teal }: { color?: string }) {
  const value = useRef(new Animated.Value(0.35)).current;
  useEffect(() => {
    const loop = Animated.loop(
      Animated.sequence([
        Animated.timing(value, { toValue: 1, duration: 700, easing: Easing.inOut(Easing.quad), useNativeDriver: true }),
        Animated.timing(value, { toValue: 0.35, duration: 700, easing: Easing.inOut(Easing.quad), useNativeDriver: true }),
      ]),
    );
    loop.start();
    return () => loop.stop();
  }, [value]);
  return <Animated.View style={[ui.dot, { backgroundColor: color, opacity: value }]} />;
}

export function StatusLine({ status, mac, problem }: { status: LinkStatus; mac: string; problem?: string }) {
  const color = status === "online" ? colors.teal : status === "connecting" ? "#f7c600" : colors.faint;
  const label = status === "online" ? `Connected to ${mac}` : status === "connecting" ? `Reaching ${mac}…` : problem ?? `${mac} is offline`;
  return (
    <View style={ui.status} accessibilityLiveRegion="polite">
      <View style={[ui.dot, { backgroundColor: color }]} />
      <Text style={ui.statusText} numberOfLines={2}>
        {label}
      </Text>
    </View>
  );
}

export function MessageView({ message, onDrawing }: { message: Message; onDrawing?: (message: Message) => void }) {
  if (message.role === "user") {
    return (
      <View style={ui.userRow}>
        <Text style={ui.user} selectable>
          {message.text}
        </Text>
      </View>
    );
  }
  const failed = message.outcome && message.outcome !== "completed";
  return (
    <View style={ui.astrid}>
      {!!message.text && (
        <Text style={[ui.astridText, failed && { color: colors.dim }]} selectable>
          {message.text}
        </Text>
      )}
      {!!message.diagram?.length && (
        <Pressable onPress={() => onDrawing?.(message)} accessibilityRole="button" accessibilityLabel="Open drawing">
          <CanvasView shapes={message.diagram} animate={false} finished interactive={false} style={ui.drawing} />
        </Pressable>
      )}
      {failed && <Text style={ui.note}>{message.outcome === "cancelled" ? "Stopped" : "Didn't finish"}</Text>}
    </View>
  );
}

/** Astrid's reply while it is being made. */
export function LiveView({ live, tall }: { live: Live; tall?: boolean }) {
  const text = live.error ?? live.answer ?? visible(live.streamed);
  const showCanvas = live.drawing || live.shapes.length > 0;
  return (
    <View style={ui.astrid}>
      {!live.finished && (
        <View style={ui.working}>
          <Pulse />
          <Text style={ui.label}>{live.drawing ? "Sketching" : live.label}</Text>
        </View>
      )}
      {!!text && <Text style={[ui.astridText, !!live.error && { color: colors.red }]}>{text}</Text>}
      {showCanvas && (
        <CanvasView shapes={live.shapes} animate finished={!live.drawing && live.finished} style={tall ? ui.drawingTall : ui.drawing} />
      )}
    </View>
  );
}

export function Composer({
  placeholder,
  busy,
  disabled,
  onSend,
  onStop,
  autoFocus,
}: {
  placeholder: string;
  busy: boolean;
  disabled?: boolean;
  onSend: (text: string) => boolean;
  onStop: () => void;
  autoFocus?: boolean;
}) {
  const [text, setText] = useState("");
  const send = () => {
    if (onSend(text)) setText("");
  };
  return (
    <View style={ui.composer}>
      <TextInput
        style={ui.input}
        value={text}
        onChangeText={setText}
        placeholder={placeholder}
        placeholderTextColor={colors.faint}
        multiline
        maxLength={4000}
        autoFocus={autoFocus}
        editable={!disabled}
        accessibilityLabel="Message to Astrid"
      />
      {busy ? (
        <Pressable style={[ui.send, { backgroundColor: "#2a2a33" }]} onPress={onStop} accessibilityRole="button" accessibilityLabel="Stop">
          <View style={ui.stop} />
        </Pressable>
      ) : (
        <Pressable
          style={[ui.send, (!text.trim() || disabled) && { opacity: 0.4 }]}
          onPress={send}
          disabled={!text.trim() || disabled}
          accessibilityRole="button"
          accessibilityLabel="Send"
        >
          <Text style={ui.arrow}>↑</Text>
        </Pressable>
      )}
    </View>
  );
}

export const ui = StyleSheet.create({
  dot: { width: 8, height: 8, borderRadius: 4 },
  status: { flexDirection: "row", alignItems: "center", gap: 8, flexShrink: 1 },
  statusText: { color: colors.dim, fontSize: 13, flexShrink: 1 },
  userRow: { alignItems: "flex-end", marginVertical: 6 },
  user: {
    maxWidth: "84%",
    color: colors.text,
    fontSize: 16,
    lineHeight: 22,
    backgroundColor: "#22232e",
    paddingHorizontal: 14,
    paddingVertical: 9,
    borderRadius: 18,
    borderBottomRightRadius: 6,
  },
  astrid: { marginVertical: 8, gap: 10 },
  astridText: { color: colors.text, fontSize: 16, lineHeight: 23 },
  drawing: { height: 240 },
  drawingTall: { flex: 1, minHeight: 260 },
  note: { color: colors.faint, fontSize: 12 },
  working: { flexDirection: "row", alignItems: "center", gap: 8 },
  label: { color: colors.dim, fontSize: 14 },
  composer: {
    flexDirection: "row",
    alignItems: "flex-end",
    gap: 8,
    backgroundColor: colors.card,
    borderRadius: 24,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: colors.line,
    paddingLeft: 16,
    paddingRight: 6,
    paddingVertical: 6,
  },
  input: { flex: 1, color: colors.text, fontSize: 16, maxHeight: 120, paddingVertical: 8 },
  send: { width: 38, height: 38, borderRadius: 19, backgroundColor: colors.blue, alignItems: "center", justifyContent: "center" },
  arrow: { color: "white", fontSize: 20, fontWeight: "700", marginTop: -2 },
  stop: { width: 12, height: 12, borderRadius: 2, backgroundColor: colors.text },
});

const arches = require("../../assets/logo-arches.png");

/**
 * Astrid's mark with living eyes, as on the web and desktop: they glance
 * around every few seconds and blink now and then. The arches are the logo's
 * own paths; the eyes sit where the logo's do, in its 20 × 16 box.
 */
export function LiveMark({ size = 64 }: { size?: number }) {
  const unit = size / 20;
  const look = useRef(new Animated.ValueXY({ x: 0, y: 0 })).current;
  const lid = useRef(new Animated.Value(1)).current;

  useEffect(() => {
    let alive = true;
    let timer: ReturnType<typeof setTimeout>;
    const glance = () => {
      if (!alive) return;
      const centre = Math.random() < 0.35;
      const angle = Math.random() * Math.PI * 2;
      Animated.spring(look, {
        toValue: centre ? { x: 0, y: 0 } : { x: Math.cos(angle) * 1.1, y: Math.sin(angle) * 0.55 },
        useNativeDriver: true,
        speed: 14,
        bounciness: 4,
      }).start();
      if (Math.random() < 0.3) {
        Animated.sequence([
          Animated.timing(lid, { toValue: 0.1, duration: 70, useNativeDriver: true }),
          Animated.timing(lid, { toValue: 1, duration: 110, useNativeDriver: true }),
        ]).start();
      }
      timer = setTimeout(glance, 1200 + Math.random() * 2200);
    };
    timer = setTimeout(glance, 600);
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [look, lid]);

  const eye = (x: number, width: number) => (
    <Animated.View
      key={x}
      style={{
        position: "absolute",
        left: x * unit,
        top: 5.49 * unit,
        width: width * unit,
        height: 4.93 * unit,
        borderRadius: 0.35 * unit,
        backgroundColor: colors.teal,
        transform: [
          { translateX: Animated.multiply(look.x, unit) },
          { translateY: Animated.multiply(look.y, unit) },
          { scaleY: lid },
        ],
      }}
    />
  );

  return (
    <View style={{ width: size, height: size * 0.8 }} accessibilityLabel="Astrid" accessible>
      <Image source={arches} style={{ width: size, height: size * 0.8 }} />
      {eye(6.01, 2.98)}
      {eye(11.02, 3.02)}
    </View>
  );
}
