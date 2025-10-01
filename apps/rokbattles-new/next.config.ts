import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const plugins = [createNextIntlPlugin()];
const isProvEnv = process.env.NODE_ENV === "production";

const config: NextConfig = {
  devIndicators: false,
  reactStrictMode: !isProvEnv,
  typedRoutes: true,
  async redirects() {
    return [
      {
        source: "/discord",
        destination: "https://discord.gg/G33SzQgx6d",
        permanent: false,
      },
      {
        source: "/desktop-app",
        destination: "https://github.com/rokbattles/rokbattles/releases",
        permanent: false,
      },
    ];
  },
};

module.exports = () => plugins.reduce((acc, next) => next(acc), config);
