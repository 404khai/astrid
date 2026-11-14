import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Pages for the phone app (ADR 0039): the live canvas and the voice ring, the
// same pages presence and the desktop use, each built into one script and one
// stylesheet with the fonts inlined, so `mobile/scripts/build-canvas.mjs` can
// fold it into a single HTML file that an Android WebView loads from the APK.
// PHONE_PAGE picks the page (canvas by default).
const page = process.env.PHONE_PAGE ?? "canvas";

export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "dist-phone",
    emptyOutDir: true,
    assetsInlineLimit: 64 * 1024 * 1024,
    cssCodeSplit: false,
    modulePreload: false,
    rollupOptions: { input: { [page]: `${page}.html` }, output: { inlineDynamicImports: true } },
  },
});
