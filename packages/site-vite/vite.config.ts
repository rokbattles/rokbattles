import mdx from "@mdx-js/rollup";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const tauri = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    { ...mdx({ include: /\/_legal\/.*\.md$/, format: "md" }), enforce: "pre" },
    react({ include: /\.(jsx|js|md|tsx|ts)$/ }),
    tailwindcss(),
  ],
  css: {
    transformer: "lightningcss",
  },
  clearScreen: tauri ? false : undefined,
  server: {
    port: 1420,
    strictPort: true,
    host: tauri || false,
    hmr: tauri
      ? {
          protocol: "ws",
          host: tauri,
          port: 1421,
        }
      : undefined,
  },
});
