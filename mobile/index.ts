import { AppRegistry, Platform } from "react-native";
import { registerRootComponent } from "expo";
import App from "./src/App";
import { AskPanel } from "./src/phone/AskPanel";

registerRootComponent(App);
// The overlay's ask panel opens in its own see-through activity (ADR 0039).
if (Platform.OS === "android") AppRegistry.registerComponent("AstridAsk", () => AskPanel);
