import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ActivityIndicator,
  AppState,
  Linking,
  BackHandler,
  FlatList,
  KeyboardAvoidingView,
  Modal,
  PermissionsAndroid,
  Pressable,
  ScrollView,
  StyleSheet,
  Switch,
  Text,
  TextInput,
  View,
} from "react-native";
import { StatusBar } from "expo-status-bar";
import { SafeAreaProvider, SafeAreaView } from "react-native-safe-area-context";
import { CameraView, useCameraPermissions } from "expo-camera";
import { overlay } from "../modules/astrid-overlay";
import { CanvasView } from "./phone/CanvasView";
import { MaterialCommunityIcons as Icon } from "@expo/vector-icons";
import { LiveMark, LiveView, Mark, MessageView, Pulse, StatusLine, colors } from "./phone/ui";
import { chatColors, chatIcons, colorOf } from "./phone/style";
import { ask, cancel, getState, load, loadDiagrams, openChat, pairFromCode, reconnect, style, unpair, useStore, type ChatRow, type Message, type Shape } from "./phone/store";

type IconName = keyof typeof Icon.glyphMap;

/** Astrid on Android (ADR 0039): pair with the Mac by QR, see and continue
 * its chats, ask, watch drawings on the live canvas, and put Astrid on top of
 * other apps. Asks run on the Mac; nothing here holds model credentials. */
export default function App() {
  const loaded = useStore((s) => s.loaded);
  const paired = useStore((s) => !!s.pairing);

  useEffect(() => {
    load();
    const sub = AppState.addEventListener("change", (next) => next === "active" && reconnect());
    // A pairing link copied from the Mac's code opens the app and pairs it.
    const pairFrom = (url: string | null) => {
      if (url?.startsWith("astrid://pair") && !getState().pairing) pairFromCode(url);
    };
    Linking.getInitialURL().then(pairFrom);
    const link = Linking.addEventListener("url", ({ url }) => pairFrom(url));
    return () => {
      sub.remove();
      link.remove();
    };
  }, []);

  return (
    <SafeAreaProvider>
      <StatusBar style="light" />
      <SafeAreaView style={styles.root} edges={["top", "bottom"]}>
        {!loaded ? <ActivityIndicator color={colors.blue} style={{ flex: 1 }} /> : paired ? <Main /> : <Pair />}
      </SafeAreaView>
    </SafeAreaProvider>
  );
}

function Pair() {
  const [permission, request] = useCameraPermissions();
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const busy = useStore((s) => s.pairingBusy);
  const handled = useRef(false);

  const scan = async () => {
    setError(null);
    if (!permission?.granted) {
      const answer = await request();
      if (!answer.granted) return setError("Astrid needs the camera to read the code on your Mac.");
    }
    handled.current = false;
    setScanning(true);
  };

  if (scanning) {
    return (
      <View style={{ flex: 1 }}>
        <CameraView
          style={{ flex: 1 }}
          facing="back"
          barcodeScannerSettings={{ barcodeTypes: ["qr"] }}
          onBarcodeScanned={async ({ data }) => {
            if (handled.current) return;
            handled.current = true;
            setScanning(false);
            const problem = await pairFromCode(data);
            if (problem) setError(problem);
          }}
        />
        <View style={styles.scanHint}>
          <Text style={styles.scanText}>Point at the code in Astrid › Settings › Phone on your Mac</Text>
          <Pressable onPress={() => setScanning(false)} style={styles.ghost} accessibilityRole="button">
            <Text style={styles.ghostText}>Cancel</Text>
          </Pressable>
        </View>
      </View>
    );
  }

  return (
    <View style={styles.pair}>
      <LiveMark size={72} />
      <Text style={styles.title}>Pair with your Mac</Text>
      <Text style={styles.body}>
        On your Mac, open Astrid, then Settings › Phone › Pair Phone, and scan the code. Your phone joins over your local
        network only. Asks run on your Mac, and your chats stay there.
      </Text>
      {busy ? (
        <ActivityIndicator color={colors.blue} />
      ) : (
        <Pressable style={styles.primary} onPress={scan} accessibilityRole="button">
          <Text style={styles.primaryText}>Scan QR code</Text>
        </Pressable>
      )}
      {!!error && <Text style={styles.error}>{error}</Text>}
    </View>
  );
}

type Tab = "new" | "diagrams";

function Main() {
  const openId = useStore((s) => s.openId);
  const chats = useStore((s) => s.chats);
  const [tab, setTab] = useState<Tab>("new");
  const [sheet, setSheet] = useState<"chats" | "settings" | null>(null);
  const [acting, setActing] = useState<ChatRow | null>(null);

  useEffect(() => {
    const sub = BackHandler.addEventListener("hardwareBackPress", () => {
      if (acting) return setActing(null), true;
      if (sheet) return setSheet(null), true;
      if (tab !== "new") return setTab("new"), true;
      if (openId) return openChat(null), true;
      return false;
    });
    return () => sub.remove();
  }, [acting, sheet, tab, openId]);

  // An ask from the overlay panel opens its chat here too.
  useEffect(() => {
    if (openId) setTab("new");
  }, [openId]);

  return (
    <View style={{ flex: 1 }}>
      {tab === "new" && !openId && <DotField />}
      <View style={styles.topBar}>
        <RoundButton icon="cog-outline" label="Settings" onPress={() => setSheet("settings")} />
        <View style={styles.tabs}>
          <TabButton icon="pencil-outline" label="New" active={tab === "new"} onPress={() => (tab === "new" ? openChat(null) : setTab("new"))} />
          <TabButton icon="view-grid-outline" label="Diagrams" active={tab === "diagrams"} onPress={() => setTab("diagrams")} />
        </View>
        <RoundButton icon="forum-outline" label="Chats" badge={chats.length} onPress={() => setSheet("chats")} />
      </View>
      {tab === "new" ? <Conversation onAct={setActing} /> : <Diagrams />}
      <Modal visible={sheet === "chats"} animationType="slide" onRequestClose={() => setSheet(null)}>
        <ChatList
          onClose={() => setSheet(null)}
          onOpen={(id) => {
            setSheet(null);
            setTab("new");
            openChat(id);
          }}
          onAct={setActing}
        />
      </Modal>
      <Modal visible={sheet === "settings"} animationType="slide" onRequestClose={() => setSheet(null)}>
        <Settings onClose={() => setSheet(null)} />
      </Modal>
      <ChatActions chat={acting} onClose={() => setActing(null)} />
    </View>
  );
}

function RoundButton({ icon, label, onPress, badge }: { icon: IconName; label: string; onPress: () => void; badge?: number }) {
  return (
    <Pressable onPress={onPress} style={({ pressed }) => [styles.round, pressed && { opacity: 0.7 }]} accessibilityRole="button" accessibilityLabel={label}>
      <Icon name={icon} size={22} color={colors.text} />
      {!!badge && (
        <View style={styles.badgeDot}>
          <Text style={styles.badgeText}>{badge > 99 ? "99+" : badge}</Text>
        </View>
      )}
    </Pressable>
  );
}

function TabButton({ icon, label, active, onPress }: { icon: IconName; label: string; active: boolean; onPress: () => void }) {
  return (
    <Pressable onPress={onPress} style={[styles.tab, active && styles.tabActive]} accessibilityRole="tab" accessibilityState={{ selected: active }} accessibilityLabel={label}>
      <Icon name={icon} size={18} color={active ? colors.text : colors.dim} />
      {active && <Text style={styles.tabText}>{label}</Text>}
    </Pressable>
  );
}

/** The faint field of coloured dots behind a new chat, fading downward. */
const DotField = memo(function DotField() {
  const rows = useMemo(() => {
    const palette = ["#4459F9", "#00F7D5", "#8A5CFF", "#E5487F", "#E8B400", "#3DBE6A", "#6b6b78"];
    return Array.from({ length: 16 }, (_, r) =>
      Array.from({ length: 26 }, (_, c) => {
        const wave = Math.sin(c * 0.45 + r * 0.2) * 3 + 8;
        const on = r < wave && Math.random() < 0.75 - r * 0.035;
        return on ? { color: palette[Math.floor(Math.random() * palette.length)], opacity: Math.max(0.12, 0.75 - r * 0.05) } : null;
      }),
    );
  }, []);
  return (
    <View style={styles.dots} pointerEvents="none">
      {rows.map((row, r) => (
        <View key={r} style={styles.dotRow}>
          {row.map((dot, c) => (
            <View key={c} style={[styles.dot, dot ? { backgroundColor: dot.color, opacity: dot.opacity } : { opacity: 0 }]} />
          ))}
        </View>
      ))}
    </View>
  );
});

function ChatIcon({ row, size = 34 }: { row?: { icon?: string; color?: string; drawings?: number }; size?: number }) {
  const tint = colorOf(row?.color) ?? colors.blue;
  const glyph = (row?.icon && chatIcons[row.icon]) || (row?.drawings ? "draw" : "message-outline");
  return (
    <View style={[styles.chatIcon, { width: size, height: size, borderRadius: size / 2, borderColor: tint + "88", backgroundColor: tint + "22" }]}>
      <Icon name={glyph as IconName} size={size * 0.5} color={tint} />
    </View>
  );
}

const ago = (iso: string) => {
  const minutes = Math.max(0, Math.round((Date.now() - Date.parse(iso)) / 60000));
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  if (minutes < 60 * 24) return `${Math.round(minutes / 60)}h`;
  return `${Math.round(minutes / 1440)}d`;
};

/** A new chat (Astrid, recent chats and the composer) or the open one. */
function Conversation({ onAct }: { onAct: (row: ChatRow) => void }) {
  const open = useStore((s) => s.open);
  const openId = useStore((s) => s.openId);
  const chats = useStore((s) => s.chats);
  const live = useStore((s) => s.live);
  const status = useStore((s) => s.status);
  const problem = useStore((s) => s.problem);
  const mac = useStore((s) => s.pairing?.mac ?? "Mac");
  const busyId = useStore((s) => s.busy);
  const busy = useStore((s) => !!s.busy || !!(s.live && !s.live.finished));
  const notice = useStore((s) => s.notice);
  const [drawing, setDrawing] = useState<Message | null>(null);
  const list = useRef<ScrollView>(null);
  const showLive = live && live.chat === openId && !open?.messages.some((m) => m.id === live.reply);
  const row = chats.find((c) => c.id === openId);

  return (
    <KeyboardAvoidingView style={{ flex: 1 }} behavior="height">
      {openId ? (
        <>
          <Pressable style={styles.threadTitle} onLongPress={() => row && onAct(row)} onPress={() => row && onAct(row)}>
            <ChatIcon row={row} size={26} />
            <Text style={styles.threadTitleText} numberOfLines={1}>
              {row?.title ?? open?.title ?? ""}
            </Text>
            <Icon name="chevron-down" size={18} color={colors.faint} />
          </Pressable>
          <ScrollView
            ref={list}
            style={{ flex: 1 }}
            contentContainerStyle={{ paddingHorizontal: 18, paddingBottom: 12 }}
            onContentSizeChange={() => list.current?.scrollToEnd({ animated: true })}
          >
            {open?.messages.map((m) => <MessageView key={m.id} message={m} onDrawing={setDrawing} />)}
            {showLive && <LiveView live={live} />}
          </ScrollView>
        </>
      ) : (
        <View style={styles.hero}>
          <View style={{ flex: 1 }} />
          <LiveMark size={76} />
          <Text style={styles.heroTitle}>What should we look at?</Text>
          <StatusLine status={status} mac={mac} problem={problem} />
          {showLive && (
            <View style={{ alignSelf: "stretch", paddingHorizontal: 18 }}>
              <LiveView live={live} />
            </View>
          )}
          <View style={{ flex: 1 }} />
          <View style={styles.recent}>
            {chats.slice(0, 2).map((c, i) => (
              <Pressable
                key={c.id}
                onPress={() => openChat(c.id)}
                onLongPress={() => onAct(c)}
                style={({ pressed }) => [styles.recentPill, i === 1 && styles.recentPillBig, pressed && { opacity: 0.8 }]}
              >
                <ChatIcon row={c} size={i === 1 ? 40 : 30} />
                <View style={{ flex: 1 }}>
                  <Text style={[styles.recentTitle, i === 1 && { fontSize: 17 }]} numberOfLines={1}>
                    {c.title}
                  </Text>
                  {i === 1 && (
                    <Text style={styles.recentSub} numberOfLines={1}>
                      {c.preview}
                    </Text>
                  )}
                </View>
                {busyId === c.id ? <Pulse /> : <Text style={styles.recentTime}>{ago(c.updated)}</Text>}
              </Pressable>
            ))}
          </View>
        </View>
      )}
      {!!notice && <Text style={[styles.error, { paddingHorizontal: 18 }]}>{notice}</Text>}
      <View style={{ paddingHorizontal: 12, paddingBottom: 10 }}>
        <PillComposer
          busy={busy}
          disabled={status !== "online"}
          placeholder={status === "online" ? "Ask, draw, explain…" : "Waiting for your Mac…"}
          onNew={() => openChat(null)}
          onSend={(text) => ask(text, openId, "app")}
          onStop={cancel}
        />
      </View>
      <DrawingModal shapes={drawing?.diagram ?? null} onClose={() => setDrawing(null)} />
    </KeyboardAvoidingView>
  );
}

function PillComposer({
  busy,
  disabled,
  placeholder,
  onNew,
  onSend,
  onStop,
}: {
  busy: boolean;
  disabled: boolean;
  placeholder: string;
  onNew: () => void;
  onSend: (text: string) => boolean;
  onStop: () => void;
}) {
  const [text, setText] = useState("");
  const ready = !!text.trim() && !disabled;
  return (
    <View style={styles.pill}>
      <Pressable style={styles.pillSide} onPress={onNew} accessibilityRole="button" accessibilityLabel="New chat">
        <Icon name="plus" size={24} color={colors.text} />
      </Pressable>
      <TextInput
        style={styles.pillInput}
        value={text}
        onChangeText={setText}
        placeholder={placeholder}
        placeholderTextColor={colors.faint}
        multiline
        maxLength={4000}
        editable={!disabled}
        accessibilityLabel="Message to Astrid"
      />
      {busy ? (
        <Pressable style={styles.pillSide} onPress={onStop} accessibilityRole="button" accessibilityLabel="Stop">
          <View style={styles.stopSquare} />
        </Pressable>
      ) : (
        <Pressable
          style={[styles.pillSide, ready && { backgroundColor: colors.blue }]}
          disabled={!ready}
          onPress={() => onSend(text) && setText("")}
          accessibilityRole="button"
          accessibilityLabel="Send"
        >
          <Icon name="arrow-up" size={22} color={ready ? "white" : colors.faint} />
        </Pressable>
      )}
    </View>
  );
}

function DrawingModal({ shapes, onClose }: { shapes: Shape[] | null; onClose: () => void }) {
  return (
    <Modal visible={!!shapes} animationType="fade" onRequestClose={onClose} statusBarTranslucent>
      <View style={styles.drawingModal}>
        {shapes && <CanvasView shapes={shapes} animate={false} finished style={{ flex: 1, borderRadius: 0 }} />}
        <Pressable style={styles.close} onPress={onClose} accessibilityLabel="Close drawing" accessibilityRole="button">
          <Icon name="close" size={24} color={colors.text} />
        </Pressable>
      </View>
    </Modal>
  );
}

/** Every drawing Astrid made, newest first, as on the desktop's Diagrams. */
function Diagrams() {
  const items = useStore((s) => s.diagrams);
  const status = useStore((s) => s.status);
  const [shown, setShown] = useState<Shape[] | null>(null);

  useEffect(() => {
    if (status === "online") loadDiagrams();
  }, [status]);

  if (!items) {
    return <View style={styles.center}>{status === "online" ? <ActivityIndicator color={colors.blue} /> : <Text style={styles.body}>Diagrams show here once your Mac is reachable.</Text>}</View>;
  }
  return (
    <>
      <FlatList
        data={items}
        keyExtractor={(d) => d.id}
        numColumns={2}
        columnWrapperStyle={{ gap: 10 }}
        contentContainerStyle={{ padding: 12, gap: 10 }}
        onRefresh={loadDiagrams}
        refreshing={false}
        ListEmptyComponent={<Text style={[styles.body, { marginTop: 80 }]}>No drawings yet. Ask Astrid to draw something.</Text>}
        renderItem={({ item }) => (
          <Pressable style={styles.tile} onPress={() => setShown(item.shapes)} accessibilityRole="button" accessibilityLabel={item.title}>
            <CanvasView shapes={item.shapes} animate={false} finished interactive={false} style={{ height: 120, borderRadius: 14 }} />
            <Text style={styles.tileTitle} numberOfLines={1}>
              {item.title}
            </Text>
            <Text style={styles.recentTime}>{ago(item.at)}</Text>
          </Pressable>
        )}
      />
      <DrawingModal shapes={shown} onClose={() => setShown(null)} />
    </>
  );
}

function ChatList({ onClose, onOpen, onAct }: { onClose: () => void; onOpen: (id: string) => void; onAct: (row: ChatRow) => void }) {
  const chats = useStore((s) => s.chats);
  const busy = useStore((s) => s.busy);
  return (
    <SafeAreaView style={styles.root} edges={["top", "bottom"]}>
      <View style={styles.sheetHeader}>
        <Text style={styles.sheetTitle}>Chats</Text>
        <RoundButton icon="close" label="Close" onPress={onClose} />
      </View>
      <FlatList
        data={chats}
        keyExtractor={(c) => c.id}
        ListEmptyComponent={<Text style={[styles.body, { marginTop: 40 }]}>No chats yet.</Text>}
        renderItem={({ item }) => (
          <Pressable
            style={({ pressed }) => [styles.row, pressed && { backgroundColor: colors.card }]}
            onPress={() => onOpen(item.id)}
            onLongPress={() => onAct(item)}
            delayLongPress={300}
          >
            <ChatIcon row={item} />
            <View style={{ flex: 1, gap: 3 }}>
              <Text style={styles.rowTitle} numberOfLines={1}>
                {item.title}
              </Text>
              <Text style={styles.rowPreview} numberOfLines={1}>
                {item.preview}
              </Text>
            </View>
            {busy === item.id ? <Pulse /> : <Text style={styles.recentTime}>{ago(item.updated)}</Text>}
          </Pressable>
        )}
      />
      <Text style={[styles.rowPreview, { textAlign: "center", padding: 10 }]}>Hold a chat to rename it or change its icon.</Text>
    </SafeAreaView>
  );
}

/** Hold a chat: rename it, or give it an icon and a colour. */
function ChatActions({ chat, onClose }: { chat: ChatRow | null; onClose: () => void }) {
  const live = useStore((s) => (chat ? s.chats.find((c) => c.id === chat.id) : undefined));
  const row = live ?? chat;
  const [title, setTitle] = useState("");
  useEffect(() => setTitle(chat?.title ?? ""), [chat?.id]);
  if (!row) return null;
  const rename = () => {
    const next = title.trim();
    if (next && next !== row.title) style(row.id, { title: next });
  };
  return (
    <Modal visible transparent animationType="fade" onRequestClose={onClose}>
      <Pressable style={styles.scrim} onPress={() => (rename(), onClose())}>
        <Pressable style={styles.sheet}>
          <View style={{ flexDirection: "row", alignItems: "center", gap: 12 }}>
            <ChatIcon row={row} size={40} />
            <TextInput
              style={styles.renameInput}
              value={title}
              onChangeText={setTitle}
              onSubmitEditing={rename}
              maxLength={80}
              returnKeyType="done"
              accessibilityLabel="Chat name"
            />
          </View>
          <Text style={styles.section}>Icon</Text>
          <View style={styles.wrap}>
            {Object.entries(chatIcons).map(([name, glyph]) => (
              <Pressable
                key={name}
                onPress={() => style(row.id, { icon: row.icon === name ? "" : name })}
                style={[styles.choice, row.icon === name && styles.choiceOn]}
                accessibilityLabel={name}
              >
                <Icon name={glyph as IconName} size={22} color={row.icon === name ? colors.text : colors.dim} />
              </Pressable>
            ))}
          </View>
          <Text style={styles.section}>Colour</Text>
          <View style={styles.wrap}>
            {chatColors.map(([name, hex]) => (
              <Pressable
                key={name}
                onPress={() => style(row.id, { color: row.color === name ? "" : name })}
                style={[styles.swatch, { backgroundColor: hex }, row.color === name && { borderColor: colors.text }]}
                accessibilityLabel={name}
              />
            ))}
          </View>
          <Pressable style={styles.primary} onPress={() => (rename(), onClose())} accessibilityRole="button">
            <Text style={styles.primaryText}>Done</Text>
          </Pressable>
        </Pressable>
      </Pressable>
    </Modal>
  );
}

const appVersion: string = require("../app.json").expo.version;

/** Settings, as on the desktop: connection, Astrid on screen, the paired Mac, About. */
function Settings({ onClose }: { onClose: () => void }) {
  const status = useStore((s) => s.status);
  const problem = useStore((s) => s.problem);
  const pairing = useStore((s) => s.pairing);
  const mac = pairing?.mac ?? "Mac";
  return (
    <SafeAreaView style={styles.root} edges={["top", "bottom"]}>
      <View style={styles.sheetHeader}>
        <Text style={styles.sheetTitle}>Settings</Text>
        <RoundButton icon="close" label="Close" onPress={onClose} />
      </View>
      <ScrollView contentContainerStyle={{ padding: 16, gap: 8, paddingBottom: 40 }}>
        <Text style={styles.section}>Mac</Text>
        <View style={styles.group}>
          <StatusLine status={status} mac={mac} problem={problem} />
          <Text style={styles.rowPreview}>Reaches {pairing?.hosts.join(", ")} on port {pairing?.port}. Asks run on your Mac, and your chats stay there.</Text>
          <Pressable style={styles.secondary} onPress={reconnect} accessibilityRole="button">
            <Text style={styles.secondaryText}>Reconnect</Text>
          </Pressable>
        </View>
        <Text style={styles.section}>On screen</Text>
        <OverlayCard />
        <Text style={styles.section}>Pairing</Text>
        <View style={styles.group}>
          <Text style={styles.rowPreview}>To pair another Mac, unpair this one first. You can also revoke this phone in Astrid › Settings › Phone on your Mac.</Text>
          <Pressable
            style={[styles.secondary, { backgroundColor: "#2a1416" }]}
            onPress={() => {
              onClose();
              overlay?.stop();
              unpair();
            }}
            accessibilityRole="button"
          >
            <Text style={[styles.secondaryText, { color: colors.red }]}>Unpair {mac}</Text>
          </Pressable>
        </View>
        <Text style={styles.section}>About</Text>
        <View style={[styles.group, { alignItems: "center", gap: 10, paddingVertical: 22 }]}>
          <LiveMark size={60} />
          <Text style={styles.title}>Astrid</Text>
          <Text style={styles.rowPreview}>Version {appVersion}</Text>
          <Text style={styles.body}>
            A personal agent that sketches, points and walks you through things. This phone holds no model keys and saves no chats: it only
            keeps the key that pairs it with your Mac.
          </Text>
          <Text style={[styles.rowPreview, { textAlign: "center" }]}>The intro wave is from Distortion Waves by Jakub Antalik (MIT).</Text>
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

/** The switch that puts Astrid's bubble on top of other apps. */
function OverlayCard() {
  const [on, setOn] = useState(() => overlay?.isRunning() ?? false);
  const [waiting, setWaiting] = useState(false);

  const turnOn = useCallback(async () => {
    if (!overlay) return;
    if (!overlay.canDrawOverlays()) {
      setWaiting(true);
      overlay.openOverlaySettings();
      return;
    }
    await PermissionsAndroid.request("android.permission.POST_NOTIFICATIONS" as never).catch(() => null);
    setOn(overlay.start(true));
  }, []);

  // Back from the system's permission page: carry on if it was allowed.
  useEffect(() => {
    const sub = AppState.addEventListener("change", (next) => {
      if (next !== "active" || !overlay) return;
      setOn(overlay.isRunning());
      if (waiting) {
        setWaiting(false);
        if (overlay.canDrawOverlays()) turnOn();
      }
    });
    return () => sub.remove();
  }, [waiting, turnOn]);

  if (!overlay) return null;
  return (
    <View style={styles.overlayCard}>
      <View style={{ flex: 1, gap: 3 }}>
        <Text style={styles.rowTitle}>Astrid on screen</Text>
        <Text style={styles.rowPreview}>
          {waiting ? "Allow “Display over other apps” for Astrid, then come back." : "A bubble over your apps. Tap it to ask."}
        </Text>
      </View>
      <Switch
        value={on}
        onValueChange={(next) => {
          if (next) turnOn();
          else {
            overlay?.stop();
            setOn(false);
          }
        }}
        trackColor={{ true: colors.blue, false: "#33333d" }}
        thumbColor="#fff"
        accessibilityLabel="Astrid on screen"
      />
    </View>
  );
}

const styles = StyleSheet.create({
  topBar: { flexDirection: "row", alignItems: "center", justifyContent: "space-between", paddingHorizontal: 16, paddingTop: 8, paddingBottom: 6 },
  round: { width: 50, height: 50, borderRadius: 25, backgroundColor: "#24242b", alignItems: "center", justifyContent: "center", borderWidth: StyleSheet.hairlineWidth, borderColor: "#3a3a44" },
  badgeDot: { position: "absolute", top: -2, right: -2, minWidth: 20, height: 20, borderRadius: 10, backgroundColor: colors.blue, alignItems: "center", justifyContent: "center", paddingHorizontal: 4 },
  badgeText: { color: "white", fontSize: 11, fontWeight: "700" },
  tabs: { flexDirection: "row", backgroundColor: "#24242b", borderRadius: 26, padding: 4, gap: 2, borderWidth: StyleSheet.hairlineWidth, borderColor: "#3a3a44" },
  tab: { flexDirection: "row", alignItems: "center", gap: 7, height: 42, paddingHorizontal: 16, borderRadius: 21 },
  tabActive: { backgroundColor: "#3a3a44" },
  tabText: { color: colors.text, fontSize: 16, fontWeight: "600" },
  dots: { position: "absolute", top: 70, left: 0, right: 0, paddingHorizontal: 12, gap: 9 },
  dotRow: { flexDirection: "row", justifyContent: "space-between" },
  dot: { width: 3, height: 3, borderRadius: 1.5 },
  hero: { flex: 1, alignItems: "center", gap: 12 },
  heroTitle: { color: colors.text, fontSize: 24, fontWeight: "700", marginTop: 8 },
  recent: { alignSelf: "stretch", paddingHorizontal: 12, gap: 8, marginBottom: 8 },
  recentPill: { flexDirection: "row", alignItems: "center", gap: 12, backgroundColor: "#24242b", borderRadius: 30, paddingVertical: 8, paddingLeft: 10, paddingRight: 18, marginHorizontal: 12, borderWidth: StyleSheet.hairlineWidth, borderColor: "#3a3a44" },
  recentPillBig: { marginHorizontal: 0, paddingVertical: 12 },
  recentTitle: { color: colors.text, fontSize: 15, fontWeight: "600" },
  recentSub: { color: colors.dim, fontSize: 13, marginTop: 2 },
  recentTime: { color: colors.faint, fontSize: 13 },
  chatIcon: { alignItems: "center", justifyContent: "center", borderWidth: 1.5 },
  threadTitle: { flexDirection: "row", alignItems: "center", gap: 10, alignSelf: "center", paddingHorizontal: 14, paddingVertical: 6, maxWidth: "88%" },
  threadTitleText: { color: colors.text, fontSize: 16, fontWeight: "600", flexShrink: 1 },
  pill: { flexDirection: "row", alignItems: "flex-end", gap: 8, backgroundColor: "#24242b", borderRadius: 30, padding: 6, borderWidth: StyleSheet.hairlineWidth, borderColor: "#3a3a44" },
  pillSide: { width: 46, height: 46, borderRadius: 23, backgroundColor: "#33333d", alignItems: "center", justifyContent: "center" },
  pillInput: { flex: 1, color: colors.text, fontSize: 17, maxHeight: 130, paddingVertical: 11 },
  stopSquare: { width: 14, height: 14, borderRadius: 3, backgroundColor: colors.text },
  center: { flex: 1, alignItems: "center", justifyContent: "center", padding: 24 },
  tile: { flex: 1, gap: 4, maxWidth: "50%" },
  tileTitle: { color: colors.text, fontSize: 14, fontWeight: "500" },
  sheetHeader: { flexDirection: "row", alignItems: "center", justifyContent: "space-between", paddingHorizontal: 20, paddingVertical: 10 },
  group: { backgroundColor: colors.card, borderRadius: 18, padding: 16, gap: 12, borderWidth: StyleSheet.hairlineWidth, borderColor: colors.line },
  secondary: { height: 42, borderRadius: 21, backgroundColor: "#2a2a33", alignItems: "center", justifyContent: "center" },
  secondaryText: { color: colors.text, fontSize: 15, fontWeight: "600" },
  renameInput: { flex: 1, color: colors.text, fontSize: 18, fontWeight: "600", borderBottomWidth: 1, borderColor: colors.line, paddingVertical: 8 },
  wrap: { flexDirection: "row", flexWrap: "wrap", gap: 10 },
  choice: { width: 44, height: 44, borderRadius: 12, alignItems: "center", justifyContent: "center", backgroundColor: "#22222a" },
  choiceOn: { backgroundColor: "#3a3a44" },
  swatch: { width: 34, height: 34, borderRadius: 17, borderWidth: 2, borderColor: "transparent" },
  root: { flex: 1, backgroundColor: colors.bg },
  pair: { flex: 1, alignItems: "center", justifyContent: "center", padding: 28, gap: 18 },
  title: { color: colors.text, fontSize: 26, fontWeight: "600" },
  body: { color: colors.dim, fontSize: 15, lineHeight: 21, textAlign: "center" },
  error: { color: colors.red, fontSize: 14, textAlign: "center" },
  primary: { backgroundColor: colors.blue, paddingHorizontal: 22, height: 48, borderRadius: 24, alignItems: "center", justifyContent: "center", alignSelf: "stretch" },
  primaryText: { color: "white", fontSize: 16, fontWeight: "600" },
  ghost: { paddingHorizontal: 18, height: 40, borderRadius: 20, backgroundColor: "rgba(255,255,255,.14)", justifyContent: "center" },
  ghostText: { color: "white", fontSize: 15 },
  scanHint: { position: "absolute", bottom: 40, left: 20, right: 20, alignItems: "center", gap: 14 },
  scanText: { color: "white", fontSize: 15, textAlign: "center", backgroundColor: "rgba(0,0,0,.55)", padding: 10, borderRadius: 12 },
  header: { flexDirection: "row", alignItems: "center", gap: 10, paddingHorizontal: 16, height: 56 },
  brand: { color: colors.text, fontSize: 20, fontWeight: "600" },
  iconButton: { width: 40, height: 40, borderRadius: 20, alignItems: "center", justifyContent: "center", backgroundColor: "rgba(255,255,255,.07)" },
  iconText: { color: colors.text, fontSize: 24, marginTop: -2 },
  statusRow: { paddingHorizontal: 20, paddingBottom: 10 },
  overlayCard: {
    padding: 16,
    borderRadius: 18,
    backgroundColor: colors.card,
    flexDirection: "row",
    alignItems: "center",
    gap: 12,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: "#2c2f6a",
  },
  section: { color: colors.faint, fontSize: 13, fontWeight: "600", letterSpacing: 0.6, textTransform: "uppercase", paddingHorizontal: 4, paddingTop: 12, paddingBottom: 2 },
  row: { flexDirection: "row", alignItems: "center", gap: 12, paddingHorizontal: 20, paddingVertical: 12, borderBottomWidth: StyleSheet.hairlineWidth, borderColor: colors.line },
  rowTitle: { color: colors.text, fontSize: 16, fontWeight: "500", flexShrink: 1 },
  rowPreview: { color: colors.dim, fontSize: 14 },
  badge: { color: colors.teal, fontSize: 11, fontWeight: "600" },
  drawings: { color: colors.faint, fontSize: 13 },
  fab: { position: "absolute", right: 18, bottom: 18, backgroundColor: colors.blue, borderRadius: 26, height: 52, paddingHorizontal: 22, justifyContent: "center", elevation: 6 },
  fabText: { color: "white", fontSize: 16, fontWeight: "600" },
  scrim: { flex: 1, backgroundColor: "rgba(0,0,0,.55)", justifyContent: "flex-end" },
  sheet: { backgroundColor: "#16161d", padding: 22, gap: 16, borderTopLeftRadius: 24, borderTopRightRadius: 24 },
  sheetTitle: { color: colors.text, fontSize: 18, fontWeight: "600" },
  empty: { alignItems: "center", gap: 14, paddingTop: 80, paddingHorizontal: 24 },
  drawingModal: { flex: 1, backgroundColor: "#141418" },
  close: { position: "absolute", top: 44, right: 16, width: 44, height: 44, borderRadius: 22, backgroundColor: "rgba(0,0,0,.5)", alignItems: "center", justifyContent: "center" },
});
