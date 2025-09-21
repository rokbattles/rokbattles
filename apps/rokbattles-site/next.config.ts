import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const plugins = [createNextIntlPlugin()];
const isProvEnv = process.env.NODE_ENV === "production";

const config: NextConfig = {
  compiler: {
    reactRemoveProperties: true,
    removeConsole: isProvEnv,
  },
  compress: true,
  eslint: {
    ignoreDuringBuilds: true,
  },
  images: {
    formats: ["image/avif", "image/webp"],
  },
  output: "standalone",
  poweredByHeader: false,
  productionBrowserSourceMaps: false,
  reactStrictMode: !isProvEnv,
  typedRoutes: true,
};

module.exports = () => plugins.reduce((acc, next) => next(acc), config);
