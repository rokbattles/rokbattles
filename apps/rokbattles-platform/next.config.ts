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
      new URL("https://imimg.lilithcdn.com/**"),
      new URL("https://imv2-gl.lilithgame.com/**"),
      new URL("https://plat-fau-global.lilithgame.com/**"),
      new URL("https://static-gl.lilithgame.com/**"),
      new URL("https://cdn.discordapp.com/**"),
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
