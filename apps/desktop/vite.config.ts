import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // Tauri expects a fixed port and exits on failure.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Don't watch the Rust source -- Tauri handles that.
      ignored: ["**/src-tauri/**"],
    },
  },
  // Tauri uses Chromium on Windows/Linux and WebKit on macOS;
  // these are sane modern targets.
  build: {
    target: "es2022",
    minify: "esbuild",
    sourcemap: true,
  },
  test: {
    environment: "jsdom",
    globals: true,
  },
});
