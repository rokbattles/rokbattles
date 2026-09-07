import { withSentryConfig } from "@sentry/nextjs/config";
import { createMDX } from "fumadocs-mdx/next";
import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin({
  experimental: {
    srcPath: "./src",
    extract: true,
    messages: {
      path: "./src/i18n/messages",
      format: "po",
      locales: "infer",
      sourceLocale: "en",
      precompile: true,
    },
  },
});

const withMDX = createMDX({});

const withSentry = (nextConfig?: NextConfig) =>
  withSentryConfig(nextConfig, {
    org: "rokbattles",
    project: "rokbattles-site",
    silent: !process.env.CI,
  });

const plugins = [withNextIntl, withMDX, withSentry];
const isProdEnv = process.env.NODE_ENV === "production";

const config: NextConfig = {
  agentRules: false,
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
      { protocol: "https", hostname: "cdn.rokbattles.com", pathname: "/**" },
    ],
  },
  output: "standalone",
  // Work around https://github.com/vercel/next.js/issues/90567, fixed by
  // https://github.com/vercel/next.js/pull/92010.
  outputFileTracingIncludes: {
    "/**": ["../../node_modules/.pnpm/@swc+helpers@*/node_modules/@swc/helpers/esm/**/*"],
  },
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
        source: "/docs",
        destination: "/docs/installation",
        permanent: false,
      },
      {
        source: "/combat-lab/new",
        destination: "/combat-lab",
        permanent: false,
      },
      {
        source: "/loot-explorer",
        destination: "/loot-explorer/barbarians",
        permanent: false,
      },
      {
        source: "/account/loot",
        destination: "/account/loot/barbarians",
        permanent: false,
      },
    ];
  },
  async rewrites() {
    return [
      {
        source: "/proxy/:path*",
        destination: `${process.env.API_URL || "http://localhost:8001"}/:path*`,
      },
    ];
  },
  async headers() {
    return [
      {
        source: "/rokbattles.mobileconfig",
        headers: [
          {
            key: "Content-Type",
            value: "application/x-apple-aspen-config",
          },
        ],
      },
      {
        source: "/:path*{/}?",
        headers: [
          {
            key: "X-Accel-Buffering",
            value: "no",
          },
        ],
      },
    ];
  },
};

module.exports = plugins.reduce((acc, next) => next(acc), config);
