import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";

// The presence app's ask surface, loaded from disk by a WKWebView (ADR 0033):
// relative asset paths, its own output folder.
export default defineConfig({
  // Tailwind only for the vendored Bklit charts (no preflight; see src/bklit/bklit.css).
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@bklit": fileURLToPath(new URL("./src/bklit", import.meta.url)) } },
  base: "./",
  build: {
    outDir: "dist-presence",
    emptyOutDir: true,
    rollupOptions: { input: { presence: "presence.html", canvas: "canvas.html", voice: "voice.html", blobs: "blobs.html", goblins: "goblins.html", group: "group.html" } },
  },
});
