import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1421,
    strictPort: true,
  },
  build: {
    // macOS WKWebView / 未来 WebView2
    target: "safari15",
    minify: "esbuild",
  },
});
