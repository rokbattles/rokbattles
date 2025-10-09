import react from "@vitejs/plugin-react";
import tsconfigPaths from "vite-tsconfig-paths";
import { defineConfig } from "vitest/config";

export default defineConfig({
  // @ts-expect-error
  plugins: [tsconfigPaths(), react()],
  test: {
    environment: "jsdom",
    include: ["src/__tests__/**/*.?(c|m)ts?(x)"],
  },
});
