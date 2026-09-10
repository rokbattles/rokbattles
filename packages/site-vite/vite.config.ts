import mdx from "@mdx-js/rollup";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import packageJson from "./package.json" with { type: "json" };

const tauri = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(packageJson.version),
  },
  plugins: [
    {
      name: "docs-package-version",
      enforce: "pre",
      transform(source, id) {
        if (!/\/content\/docs\/.*\.mdx?$/.test(id.split("?")[0])) return;
        if (!source.includes("__VERSION__")) return;
        return { code: source.replaceAll("__VERSION__", packageJson.version), map: null };
      },
    },
    { ...mdx({ include: /\/content\/(?:legal|docs)\/.*\.mdx?$/ }), enforce: "pre" },
    react({ include: /\.(jsx|js|mdx?|tsx|ts)$/ }),
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
