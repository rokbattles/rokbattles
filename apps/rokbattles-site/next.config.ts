import type { NextConfig } from "next";

const isProvEnv = process.env.NODE_ENV === "production";

const config: NextConfig = {
  output: "standalone",
  reactStrictMode: !isProvEnv,
};

export default config;
