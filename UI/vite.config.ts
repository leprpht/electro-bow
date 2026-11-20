import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  // The bundle is served from JUCE's in-memory resource provider rather than
  // a real site root. Relative asset URLs work consistently across WebView2,
  // WebKit, and hosts that rewrite the resource-provider URL.
  base: "./",
  plugins: [react(), tailwindcss()],
  build: {
    // The native resource provider serves only HTML, JS and CSS. Keep fonts
    // and faceplate textures inside those resources for fully offline loading.
    assetsInlineLimit: 512 * 1024,
    outDir: "dist",
    emptyOutDir: true,
    target: "es2020",
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        entryFileNames: "app.js",
        assetFileNames: (asset) =>
          asset.names?.some((name) => name.endsWith(".css")) ? "app.css" : "[name][extname]",
      },
    },
  },
});
