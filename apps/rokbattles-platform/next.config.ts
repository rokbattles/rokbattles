import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin();

const plugins = [withNextIntl];
const isProdEnv = process.env.NODE_ENV === "production";

const config: NextConfig = {
  compiler: {
    reactRemoveProperties: true,
    removeConsole: isProdEnv,
  },
  compress: true,
  devIndicators: false,
  experimental: {
    typedEnv: true,
  },
  images: {
    formats: ["image/avif", "image/webp"],
    remotePatterns: [
      { protocol: "https", hostname: "imimg.lilithcdn.com", pathname: "/**" },
      { protocol: "https", hostname: "imv2-gl.lilithgame.com", pathname: "/**" },
      { protocol: "https", hostname: "plat-fau-global.lilithgame.com", pathname: "/**" },
      { protocol: "https", hostname: "static-gl.lilithgame.com", pathname: "/**" },
      { protocol: "https", hostname: "cdn.discordapp.com", pathname: "/**" },
    ],
  },
  output: "standalone",
  productionBrowserSourceMaps: false,
  reactStrictMode: !isProdEnv,
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
      {
        source: "/my-reports",
        destination: "/account/reports",
        permanent: false,
      },
      {
        source: "/my-pairings",
        destination: "/account/pairings",
        permanent: false,
      },
    ];
  },
};

module.exports = plugins.reduce((acc, next) => next(acc), config);
