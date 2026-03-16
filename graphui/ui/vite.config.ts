import path from "path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// https://vite.dev/config/
export default defineConfig({
  plugins: [tailwindcss(), react()],
  base: "/",
  resolve: {
    alias: {
      // Maps @docs/* to <repo-root>/docs/*
      "@docs": path.resolve(__dirname, "../../docs"),
    },
  },
  server: {
    fs: {
      // Allow serving files from the repo root (needed for docs/ outside graphui/ui/)
      allow: [path.resolve(__dirname, "../..")],
    },
  },
  build: {
    outDir: "dist",
    rollupOptions: {
      output: {
        // Predictable filenames so Rust include_str! paths are stable.
        entryFileNames: "assets/app.js",
        chunkFileNames: "assets/[name].js",
        assetFileNames: (info) =>
          info.name?.endsWith(".css") ? "assets/app.css" : "assets/[name][extname]",
      },
    },
  },
});
