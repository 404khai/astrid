import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  build: {
    rollupOptions: {
      // Two pages: the homepage and the docs (overview, brand, components).
      input: { main: "index.html", docs: "docs.html" },
    },
  },
});
