import { Platform } from "react-native";
import { requireOptionalNativeModule } from "expo-modules-core";

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
};

export const overlay: Overlay | null = Platform.OS === "android" ? requireOptionalNativeModule<Overlay>("AstridOverlay") : null;
