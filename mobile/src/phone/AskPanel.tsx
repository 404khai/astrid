import { useEffect } from "react";
import { BackHandler, KeyboardAvoidingView, Pressable, ScrollView, StyleSheet, Text, View } from "react-native";
import { overlay } from "../../modules/astrid-overlay";
import { Composer, LiveView, Mark, StatusLine, colors } from "./ui";
import { ask, cancel, load, newPanelChat, reconnect, useStore } from "./store";

/**
 * The ask panel that opens over other apps from Astrid's bubble (ADR 0039).
 * Same link and state as the app: an ask here runs on the Mac and is saved in
 * its chats. Follow-ups stay in the panel's chat until you start a new one.
 * Tapping above the panel closes it; what was on screen stays as it was.
 */
export function AskPanel() {
  const status = useStore((s) => s.status);
  const problem = useStore((s) => s.problem);
  const mac = useStore((s) => s.pairing?.mac ?? "your Mac");
  const paired = useStore((s) => !!s.pairing);
  const panelChat = useStore((s) => s.panelChat);
  const live = useStore((s) => s.live);
  const notice = useStore((s) => s.notice);
  const busy = useStore((s) => !!s.busy || !!(s.live && !s.live.finished));
  const shown = live && (live.chat === panelChat || (!panelChat && !live.finished)) ? live : null;
  const close = () => overlay?.closePanel();

  useEffect(() => {
    load();
    reconnect();
    const sub = BackHandler.addEventListener("hardwareBackPress", () => {
      close();
      return true;
    });
    return () => sub.remove();
  }, []);

  return (
    <KeyboardAvoidingView style={styles.root} behavior="height">
      <Pressable style={{ flex: 1 }} onPress={close} accessibilityLabel="Close Astrid" />
      <View style={[styles.panel, shown && (shown.drawing || shown.shapes.length > 0) && styles.tall]}>
        <View style={styles.grip} />
        <View style={styles.header}>
          <Mark size={26} />
          <View style={{ flex: 1 }}>
            {paired ? <StatusLine status={status} mac={mac} problem={problem} /> : <Text style={styles.dim}>Open Astrid to pair with your Mac first.</Text>}
          </View>
          {!!panelChat && !busy && (
            <Pressable onPress={newPanelChat} style={styles.chip} accessibilityRole="button">
              <Text style={styles.chipText}>New</Text>
            </Pressable>
          )}
        </View>
        {shown && (
          <ScrollView style={{ flexGrow: 0, flexShrink: 1 }} contentContainerStyle={{ flexGrow: 1 }}>
            <LiveView live={shown} tall />
          </ScrollView>
        )}
        {!!notice && <Text style={styles.notice}>{notice}</Text>}
        <Composer
          placeholder={status === "online" ? "Ask Astrid" : "Waiting for your Mac…"}
          busy={busy}
          disabled={status !== "online"}
          autoFocus
          onSend={(text) => ask(text, panelChat, "panel")}
          onStop={cancel}
        />
      </View>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: "transparent" },
  panel: {
    marginHorizontal: 10,
    marginBottom: 10,
    padding: 14,
    gap: 12,
    borderRadius: 28,
    backgroundColor: "rgba(16,16,22,0.97)",
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: "#30348a",
    maxHeight: "86%",
    elevation: 16,
  },
  tall: { height: "72%" },
  grip: { alignSelf: "center", width: 36, height: 4, borderRadius: 2, backgroundColor: "#3a3a44", marginTop: -4 },
  header: { flexDirection: "row", alignItems: "center", gap: 10 },
  dim: { color: colors.dim, fontSize: 13 },
  chip: { paddingHorizontal: 12, height: 30, borderRadius: 15, backgroundColor: "rgba(255,255,255,.08)", justifyContent: "center" },
  chipText: { color: colors.text, fontSize: 13 },
  notice: { color: colors.red, fontSize: 13 },
});
