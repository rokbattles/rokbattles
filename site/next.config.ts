import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin({
  experimental: {
    srcPath: "./src",
    extract: {
      sourceLocale: "en",
    },
    messages: {
      path: "./src/i18n/messages",
      format: "po",
      locales: "infer",
      precompile: true,
    },
  },
});

const plugins = [withNextIntl];
const isProdEnv = process.env.NODE_ENV === "production";

const config: NextConfig = {
  compiler: {
    reactRemoveProperties: true,
    removeConsole: isProdEnv,
  },
  compress: true,
  experimental: {
    typedEnv: true,
  },
  images: {
    unoptimized: true,
    remotePatterns: [
      { protocol: "https", hostname: "imimg.lilithcdn.com", pathname: "/**" },
      { protocol: "https", hostname: "imv2-gl.lilithgame.com", pathname: "/**" },
      { protocol: "https", hostname: "plat-fau-global.lilithgame.com", pathname: "/**" },
      { protocol: "https", hostname: "static-gl.lilithgame.com", pathname: "/**" },
      { protocol: "https", hostname: "cdn.discordapp.com", pathname: "/**" },
      { protocol: "https", hostname: "cdn.rokbattles.com", pathname: "/**" },
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
    ];
  },
};

module.exports = plugins.reduce((acc, next) => next(acc), config);
