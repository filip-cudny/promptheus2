import path from "path";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://v2.tauri.app/start/frontend/svelte/
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: path.resolve("./src/lib"),
    },
  },
  clearScreen: false,
  server: {
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "esnext",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
      input: {
        main: path.resolve("./index.html"),
        "context-menu": path.resolve("./context-menu.html"),
        "prompt-dialog": path.resolve("./prompt-dialog.html"),
        "context-editor": path.resolve("./context-editor.html"),
        "image-preview": path.resolve("./image-preview.html"),
        "text-preview": path.resolve("./text-preview.html"),
      },
    },
  },
});
