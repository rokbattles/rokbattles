import type { NextConfig } from "next";

const plugins = [];
const isProvEnv = process.env.NODE_ENV === "production";

const config: NextConfig = {
  compiler: {
    reactRemoveProperties: true,
    removeConsole: isProvEnv,
  },
  compress: true,
  devIndicators: false,
  images: {
    formats: ["image/avif", "image/webp"],
  },
  output: "standalone",
  productionBrowserSourceMaps: false,
  reactStrictMode: !isProvEnv,
};

module.exports = () => plugins.reduce((acc, next) => next(acc), config);
