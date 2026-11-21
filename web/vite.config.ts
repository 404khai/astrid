import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  build: {
    rollupOptions: {
      // One page: main.tsx routes /, /docs and /changelog.
      input: { main: "index.html" },
    },
  },
});
