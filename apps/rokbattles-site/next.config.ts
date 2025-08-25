import type { NextConfig } from "next";

const isProvEnv = process.env.NODE_ENV === "production";

export const config: NextConfig = {
  output: "standalone",
  reactStrictMode: !isProvEnv,
};
