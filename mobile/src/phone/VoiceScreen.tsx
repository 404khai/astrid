import { useEffect, useRef, useState } from "react";
import { KeyboardAvoidingView, Modal, PermissionsAndroid, Pressable, StyleSheet, Text, TextInput, View, useWindowDimensions } from "react-native";
import { MaterialCommunityIcons as Icon } from "@expo/vector-icons";
import { SafeAreaView } from "react-native-safe-area-context";
import { overlay, type VoiceEvent } from "../../modules/astrid-overlay";
import { ask, cancel, getState, useStore, visible } from "./store";
import { VoiceRing, type RingState } from "./WebPages";
import { colors } from "./ui";

type Phase = "listening" | "thinking" | "speaking" | "idle";

/** What Astrid says out loud: the gist of the reply, or what she drew. */
function spoken(): string {
  const live = getState().live;
  if (!live) return "";
  const text = (live.error ?? live.answer ?? visible(live.streamed))
    .replace(/\*\*|__|`|#+\s|^\s*[-*]\s|^\s*\d+\.\s/gm, "")
    .replace(/\s+/g, " ")
    .trim();
  if (text) {
    if (text.length <= 600) return text;
    const cut = text.slice(0, 600);
    return cut.slice(0, Math.max(cut.lastIndexOf(". ") + 1, 300));
  }
  const title = live.shapes.find((s) => s.shape === "title")?.text;
  return title ? `Here's ${title}.` : live.shapes.length ? "Here's the drawing." : "";
}

/**
 * Voice mode on the phone (ADR 0039 amendment), like ChatGPT's: Astrid in
 * her voice ring in the middle of a black screen. Talk, and what you say is
 * asked in the open chat when you stop; her reply is spoken, then she
 * listens again. Mute keeps her talking but stops listening; × leaves.
 */
export function VoiceScreen({ visible: open, onClose, onMenu, onSettings }: { visible: boolean; onClose: () => void; onMenu: () => void; onSettings: () => void }) {
  const { width } = useWindowDimensions();
  const openId = useStore((s) => s.openId);
  const live = useStore((s) => s.live);
  const status = useStore((s) => s.status);
  const [phase, setPhase] = useState<Phase>("idle");
  const [level, setLevel] = useState(0);
  const [heard, setHeard] = useState("");
  const [note, setNote] = useState<string | null>(null);
  const [muted, setMuted] = useState(false);
  const [text, setText] = useState("");
  const phaseRef = useRef(phase);
  phaseRef.current = phase;
  const mutedRef = useRef(muted);
  mutedRef.current = muted;
  const reply = useRef<string | null>(null);

  const listen = () => {
    setHeard("");
    setLevel(0);
    setPhase("listening");
    if (!mutedRef.current) overlay?.listen();
  };

  const send = (said: string) => {
    if (!ask(said, getState().openId, "app")) {
      setNote(status === "online" ? "Astrid is busy." : "Not connected to your Mac.");
      return listen();
    }
    reply.current = null;
    setPhase("thinking");
  };

  // Open: ask for the mic, then listen. Close: stop everything.
  useEffect(() => {
    if (!open) return;
    let alive = true;
    setNote(null);
    setMuted(false);
    PermissionsAndroid.request(PermissionsAndroid.PERMISSIONS.RECORD_AUDIO).then((answer) => {
      if (!alive) return;
      if (answer !== PermissionsAndroid.RESULTS.GRANTED) {
        setNote("Astrid needs the microphone to listen. You can still type below.");
        setPhase("idle");
      } else listen();
    });
    const sub = overlay?.addListener("onVoice", (event: VoiceEvent) => {
      switch (event.type) {
        case "listening":
          if (!event.onDevice) setNote("This phone recognises speech with its system service, which may use the internet.");
          break;
        case "level":
          if (phaseRef.current === "listening") setLevel(event.level);
          break;
        case "partial":
          if (phaseRef.current === "listening") setHeard(event.text);
          break;
        case "final":
          if (phaseRef.current !== "listening") break;
          if (event.text.trim()) send(event.text.trim());
          else listen();
          break;
        case "silence":
          if (phaseRef.current === "listening") listen();
          break;
        case "error":
          setNote(event.message);
          setPhase("idle");
          break;
        case "speaking":
          setPhase("speaking");
          break;
        case "spoken":
          if (phaseRef.current === "speaking") listen();
          break;
      }
    });
    return () => {
      alive = false;
      sub?.remove();
      overlay?.stopListening();
      overlay?.hush();
      setPhase("idle");
    };
  }, [open]);

  // The reply to our ask is finished: say it.
  useEffect(() => {
    if (!open || phaseRef.current !== "thinking" || !live) return;
    reply.current = live.reply;
    if (!live.finished) return;
    const words = spoken();
    if (words) {
      setPhase("speaking");
      overlay?.say(words);
    } else listen();
  }, [open, live?.reply, live?.finished]);

  const toggleMute = () => {
    const next = !muted;
    setMuted(next);
    mutedRef.current = next;
    if (next) {
      overlay?.stopListening();
      setLevel(0);
    } else if (phase === "listening" || phase === "idle") listen();
  };

  const interrupt = () => {
    if (phase === "speaking") {
      overlay?.hush();
      listen();
    } else if (phase === "thinking") cancel();
    else if (phase === "idle") listen();
  };

  const ring: RingState = phase === "listening" && !muted ? "listening" : phase === "speaking" ? "speaking" : phase === "thinking" ? "processing" : "idle";
  const caption =
    phase === "listening" ? (muted ? "Muted" : heard || "Listening…") : phase === "thinking" ? live?.label ?? "Thinking…" : phase === "speaking" ? "Tap Astrid to interrupt" : "";
  const size = Math.min(width * 0.78, 340);

  return (
    <Modal visible={open} animationType="fade" onRequestClose={onClose} statusBarTranslucent>
      <SafeAreaView style={styles.root} edges={["top", "bottom"]}>
        <KeyboardAvoidingView style={{ flex: 1 }} behavior="height">
          <View style={styles.top}>
            <Pressable style={styles.round} onPress={onMenu} accessibilityLabel="Open sidebar">
              <Icon name="menu" size={24} color={colors.text} />
            </Pressable>
            <Pressable style={styles.round} onPress={onSettings} accessibilityLabel="Settings">
              <Icon name="tune-variant" size={22} color={colors.text} />
            </Pressable>
          </View>
          <View style={styles.middle}>
            <VoiceRing state={ring} level={muted ? 0 : level} size={size} onTap={interrupt} />
            <Text style={styles.caption} numberOfLines={3} accessibilityLiveRegion="polite">
              {caption}
            </Text>
            {!!note && <Text style={styles.note}>{note}</Text>}
            {!openId && <Text style={styles.note}>A new chat starts with what you say.</Text>}
          </View>
          <View style={styles.bottom}>
            <View style={styles.input}>
              <Icon name="plus" size={24} color={colors.text} />
              <TextInput
                style={styles.field}
                value={text}
                onChangeText={setText}
                placeholder="Ask Astrid"
                placeholderTextColor={colors.faint}
                returnKeyType="send"
                onSubmitEditing={() => {
                  if (!text.trim()) return;
                  overlay?.stopListening();
                  send(text.trim());
                  setText("");
                }}
              />
            </View>
            <Pressable style={[styles.side, muted && { backgroundColor: "#4a1d20" }]} onPress={toggleMute} accessibilityLabel={muted ? "Unmute" : "Mute"}>
              <Icon name={muted ? "microphone-off" : "microphone"} size={24} color={colors.text} />
            </Pressable>
            <Pressable style={[styles.side, styles.close]} onPress={onClose} accessibilityLabel="End voice">
              <Icon name="close" size={26} color="#000" />
            </Pressable>
          </View>
        </KeyboardAvoidingView>
      </SafeAreaView>
    </Modal>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: "#000" },
  top: { flexDirection: "row", justifyContent: "space-between", paddingHorizontal: 16, paddingTop: 10 },
  round: { width: 50, height: 50, borderRadius: 25, backgroundColor: "#262629", alignItems: "center", justifyContent: "center" },
  middle: { flex: 1, alignItems: "center", justifyContent: "center", gap: 14, paddingHorizontal: 24 },
  caption: { color: colors.text, fontSize: 18, textAlign: "center", minHeight: 26 },
  note: { color: colors.faint, fontSize: 13, textAlign: "center" },
  bottom: { flexDirection: "row", alignItems: "center", gap: 10, paddingHorizontal: 14, paddingBottom: 10 },
  input: { flex: 1, flexDirection: "row", alignItems: "center", gap: 10, height: 56, borderRadius: 28, backgroundColor: "#262629", paddingHorizontal: 16 },
  field: { flex: 1, color: colors.text, fontSize: 17 },
  side: { width: 56, height: 56, borderRadius: 28, backgroundColor: "#262629", alignItems: "center", justifyContent: "center" },
  close: { backgroundColor: "#fff" },
});
