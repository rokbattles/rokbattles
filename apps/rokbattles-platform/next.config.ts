import type { NextConfig } from "next";

const plugins = [];
const isProdEnv = process.env.NODE_ENV === "production";

const config: NextConfig = {
  compiler: {
    reactRemoveProperties: true,
    removeConsole: isProdEnv,
  },
  compress: true,
  devIndicators: false,
  eslint: {
    ignoreDuringBuilds: true,
  },
  experimental: {
    typedEnv: true,
  },
  images: {
    formats: ["image/avif", "image/webp"],
    remotePatterns: [
      new URL("https://imimg.lilithcdn.com/**"),
      new URL("https://imv2-gl.lilithgame.com/**"),
      new URL("https://plat-fau-global.lilithgame.com/**"),
      new URL("https://static-gl.lilithgame.com/**"),
    ],
  },
  productionBrowserSourceMaps: false,
  reactStrictMode: !isProdEnv,
};

module.exports = plugins.reduce((acc, next) => next(acc), config);
