import { Platform } from "react-native";
import { requireOptionalNativeModule, type EventSubscription } from "expo-modules-core";

/** Voice mode's events (ADR 0039 amendment). */
export type VoiceEvent =
  | { type: "listening"; onDevice: boolean }
  | { type: "level"; level: number }
  | { type: "partial" | "final"; text: string }
  | { type: "silence" | "error"; message: string }
  | { type: "speaking" | "spoken" };

/** Astrid's Android overlay (ADR 0039): the bubble over other apps, its ask
 * panel and the intro wave. Absent on iOS, which has no cross-app overlay. */
type Overlay = {
  canDrawOverlays(): boolean;
  openOverlaySettings(): void;
  isRunning(): boolean;
  start(wave: boolean): boolean;
  stop(): void;
  playWave(): void;
  closePanel(): void;
  listen(): void;
  stopListening(): void;
  say(text: string): void;
  hush(): void;
  addListener(event: "onVoice", listener: (event: VoiceEvent) => void): EventSubscription;
};

export const overlay: Overlay | null = Platform.OS === "android" ? requireOptionalNativeModule<Overlay>("AstridOverlay") : null;
