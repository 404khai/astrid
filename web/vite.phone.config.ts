import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The live canvas for the phone app (ADR 0039): the same page as the presence
// panel's, built into one script and one stylesheet with the fonts inlined, so
// `mobile/scripts/build-canvas.mjs` can fold it into a single HTML file that an
// Android WebView loads from the APK's assets.
export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "dist-phone",
    emptyOutDir: true,
    assetsInlineLimit: 64 * 1024 * 1024,
    cssCodeSplit: false,
    modulePreload: false,
    rollupOptions: { input: { canvas: "canvas.html" }, output: { inlineDynamicImports: true } },
  },
});
