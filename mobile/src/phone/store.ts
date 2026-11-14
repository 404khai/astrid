import { useSyncExternalStore } from "react";
import { Platform } from "react-native";
import * as SecureStore from "expo-secure-store";
import { getRandomBytes } from "expo-crypto";
import { LinkClient, pair as pairWithMac, type LinkDeps, type LinkStatus, type Pairing } from "../link/client.ts";
import { parsePairing } from "../link/pairing.ts";

/** The phone app's state (ADR 0039), shared by the app and the overlay's ask
 * panel: both run on one React runtime, so they share one link to the Mac.
 * Chats live on the Mac; the phone keeps them in memory only. */
export type Shape = Record<string, unknown> & { shape: string };
export type ChatRow = { id: string; title: string; updated: string; preview: string; drawings: number; icon?: string; color?: string };
/** A saved drawing, from the Mac's gallery. */
export type Diagram = { id: string; chat: string; title: string; at: string; shapes: Shape[] };
export type Message = {
  id: string;
  role: "user" | "astrid";
  text: string;
  brief?: Record<string, unknown>;
  diagram?: Shape[];
  outcome?: string;
  at: string;
};
export type Chat = { id: string; title: string; messages: Message[] };
/** The ask running on the Mac, from the phone or the desktop. */
export type Live = {
  chat: string;
  reply: string;
  streamed: string;
  label: string;
  shapes: Shape[];
  drawing: boolean;
  finished: boolean;
  answer?: string;
  error?: string;
};

export type State = {
  loaded: boolean;
  pairing: Pairing | null;
  status: LinkStatus;
  problem?: string;
  pairingBusy: boolean;
  chats: ChatRow[];
  busy?: string;
  openId: string | null;
  open: Chat | null;
  live: Live | null;
  notice?: string;
  /** The chat the overlay panel is in; a new one until its first ask. */
  panelChat: string | null;
  diagrams: Diagram[] | null;
};

const KEY = "astrid.pairing";
let state: State = {
  loaded: false,
  pairing: null,
  status: "offline",
  pairingBusy: false,
  chats: [],
  openId: null,
  open: null,
  live: null,
  panelChat: null,
  diagrams: null,
};
const listeners = new Set<() => void>();
let client: LinkClient | null = null;
/** Set when an ask is sent, until the Mac says where it went. */
let pendingFrom: "app" | "panel" | null = null;

function set(next: Partial<State>) {
  state = { ...state, ...next };
  listeners.forEach((l) => l());
}

export function useStore<T>(pick: (s: State) => T): T {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => pick(state),
  );
}

export const getState = () => state;

const deps: LinkDeps = {
  random: (n) => getRandomBytes(n),
  socket: (url) => new WebSocket(url) as unknown as ReturnType<LinkDeps["socket"]>,
};

/** The visible part of a streamed reply: before its first fenced block. */
export const visible = (text: string) => (text.split("```")[0] ?? "").trim();

export async function load() {
  if (state.loaded) return;
  let pairing: Pairing | null = null;
  try {
    const raw = await SecureStore.getItemAsync(KEY);
    if (raw) pairing = JSON.parse(raw);
  } catch {}
  set({ loaded: true, pairing });
  if (pairing) connect(pairing);
}

function connect(pairing: Pairing) {
  client?.stop();
  client = new LinkClient(pairing, deps);
  client.onStatus = (status, problem) => set({ status, problem: status === "online" ? undefined : problem });
  client.onMessage = receive;
  client.start();
  if (state.openId) setTimeout(() => client?.send({ type: "open", chat: state.openId }), 0);
}

/** Pairs from the text of a scanned QR code. */
export async function pairFromCode(text: string): Promise<string | null> {
  const offer = parsePairing(text);
  if (!offer) return "That isn't an Astrid pairing code.";
  set({ pairingBusy: true });
  try {
    const constants = Platform.constants as { Model?: string; Brand?: string };
    const name = [constants.Brand, constants.Model].filter(Boolean).join(" ") || "Android phone";
    const pairing = await pairWithMac(offer, name, deps);
    await SecureStore.setItemAsync(KEY, JSON.stringify(pairing));
    set({ pairing, pairingBusy: false });
    connect(pairing);
    return null;
  } catch (error) {
    set({ pairingBusy: false });
    return error instanceof Error ? error.message : "Pairing failed.";
  }
}

export async function unpair() {
  client?.stop();
  client = null;
  await SecureStore.deleteItemAsync(KEY).catch(() => {});
  set({ pairing: null, chats: [], open: null, openId: null, live: null, status: "offline", panelChat: null });
}

export function reconnect() {
  client?.nudge();
}

export function openChat(id: string | null) {
  set({ openId: id, open: id && state.open?.id === id ? state.open : null, notice: undefined });
  if (id) client?.send({ type: "open", chat: id });
}

/** Asks Astrid on the Mac. `chat` null starts a new chat. */
export function ask(text: string, chat: string | null, from: "app" | "panel"): boolean {
  const trimmed = text.trim();
  if (!trimmed || state.busy || (state.live && !state.live.finished)) return false;
  if (!client?.send({ type: "ask", chat, text: trimmed })) {
    set({ notice: "Not connected to your Mac." });
    return false;
  }
  pendingFrom = from;
  set({ notice: undefined });
  return true;
}

/** Renames a chat or changes its icon or colour (an empty string resets). */
export function style(chat: string, change: { title?: string; icon?: string; color?: string }) {
  if (!client?.send({ type: "style", chat, ...change })) set({ notice: "Not connected to your Mac." });
}

/** Asks the Mac for its newest drawings. */
export function loadDiagrams() {
  client?.send({ type: "diagrams" });
}

export function cancel() {
  client?.send({ type: "cancel" });
}

export function newPanelChat() {
  set({ panelChat: null });
}

function receive(message: Record<string, unknown>) {
  switch (message.type) {
    case "chats":
      set({ chats: (message.chats as ChatRow[]) ?? [], busy: (message.busy as string) ?? undefined });
      return;
    case "diagrams":
      set({ diagrams: ((message.items as Diagram[]) ?? []).slice(0, 40) });
      return;
    case "chat": {
      const chat = message.chat as Chat;
      if (!chat || chat.id !== state.openId) return;
      // The saved reply replaces the live one once it is in the chat.
      const live = state.live && state.live.finished && chat.messages.some((m) => m.id === state.live?.reply) ? null : state.live;
      set({ open: chat, live });
      return;
    }
    case "started": {
      const chatId = String(message.chat);
      const from = pendingFrom;
      pendingFrom = null;
      set({
        live: { chat: chatId, reply: String(message.reply), streamed: "", label: "Thinking", shapes: [], drawing: false, finished: false },
        ...(from === "panel" ? { panelChat: chatId } : {}),
      });
      // Open where it went: a new chat gets its ID only now.
      if (state.openId !== chatId) openChat(chatId);
      return;
    }
    case "live":
      return apply(String(message.chat), String(message.reply), message.event as Record<string, unknown>);
    case "refused":
      pendingFrom = null;
      set({ notice: String(message.message ?? "The Mac refused.") });
      return;
  }
}

function apply(chat: string, reply: string, event: Record<string, unknown>) {
  let live = state.live;
  if (!live || live.reply !== reply) {
    // An ask from the desktop: watch it too.
    live = { chat, reply, streamed: "", label: "Thinking", shapes: [], drawing: false, finished: false };
  }
  live = { ...live };
  switch (event.type) {
    case "state":
      if (event.state === "idle") live.finished = true;
      else live.label = String(event.label ?? live.label);
      break;
    case "delta":
      live.streamed += String(event.text ?? "");
      break;
    case "canvas":
      if (event.event === "begin") {
        live.shapes = [];
        live.drawing = true;
      } else if (event.event === "shape" && event.shape && live.shapes.length < 200) {
        live.shapes = [...live.shapes, event.shape as Shape];
      } else if (event.event === "end") {
        live.drawing = false;
      } else if (event.event === "cancel") {
        live.drawing = false;
        live.shapes = [];
      }
      break;
    case "answer":
      live.answer = String(event.text ?? "");
      break;
    case "error":
      live.error = String(event.message ?? "Something went wrong.");
      break;
    case "done":
      live.finished = true;
      break;
  }
  set({ live });
}
