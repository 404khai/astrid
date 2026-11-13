import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The presence app's ask surface, loaded from disk by a WKWebView (ADR 0033):
// relative asset paths, its own output folder.
export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "dist-presence",
    emptyOutDir: true,
    rollupOptions: { input: { presence: "presence.html" } },
  },
});
