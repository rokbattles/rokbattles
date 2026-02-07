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

const config: NextConfig = {
  images: {
    formats: ["image/avif", "image/webp"],
    remotePatterns: [
      // new URL("https://imimg.lilithcdn.com/**"),
      // new URL("https://imv2-gl.lilithgame.com/**"),
      // new URL("https://plat-fau-global.lilithgame.com/**"),
      // new URL("https://static-gl.lilithgame.com/**"),
      // new URL("https://cdn.discordapp.com/**"),
      {
        protocol: "https",
        hostname: "cdn.discordapp.com",
        port: "",
        pathname: "**",
      },
    ],
  },
};

module.exports = plugins.reduce((acc, next) => next(acc), config);
