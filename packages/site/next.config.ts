import createMDX from "@next/mdx";
import { withSentryConfig } from "@sentry/nextjs/config";
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

const withMDX = createMDX({ extension: /\.mdx?$/ });

const withSentry = (nextConfig?: NextConfig) =>
  withSentryConfig(nextConfig, {
    org: "rokbattles",
    project: "rokbattles-site",
    silent: !process.env.CI,
  });

const plugins = [withNextIntl, withMDX, withSentry];
const isProdEnv = process.env.NODE_ENV === "production";
const isDevEnv = process.env.NODE_ENV === "development";

// Images are unoptimized, so these are browser image sources, not optimizer rules.
const imageSources = [
  "https://imimg.lilithcdn.com",
  "https://imv2-gl.lilithgame.com",
  "https://plat-fau-global.lilithgame.com",
  "https://static-gl.lilithgame.com",
  "https://cdn.rokbattles.com",
];

function sentryConnectSources(): string[] {
  const dsn = process.env.NEXT_PUBLIC_SENTRY_DSN;
  if (!dsn?.trim()) return [];
  let url: URL;
  try {
    url = new URL(dsn);
  } catch {
    throw new Error("NEXT_PUBLIC_SENTRY_DSN must be an absolute HTTP(S) URL");
  }
  if (url.protocol !== "https:" && url.protocol !== "http:") {
    throw new Error("NEXT_PUBLIC_SENTRY_DSN must be an absolute HTTP(S) URL");
  }
  // Regional/custom ingest hosts work without exposing the DSN key/project.
  return [url.origin];
}

// Static-compatible baseline. Next's RSC/hydration and the theme bootstrap need
// inline scripts; a strict nonce policy would require request-time rendering and
// changing HTML caching. This deliberately does not prevent all inline-script XSS.
const contentSecurityPolicy = [
  "default-src 'self'",
  `script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'${isDevEnv ? " 'unsafe-eval'" : ""}`,
  "script-src-attr 'none'",
  // React, charts, motion, and UI primitives rely on inline style attributes.
  "style-src 'self' 'unsafe-inline'",
  `img-src 'self' data: ${imageSources.join(" ")}`,
  // next/font downloads at build time and serves the font files from this origin.
  "font-src 'self'",
  `connect-src ${[
    "'self'",
    "https://cdn.rokbattles.com", // Map binary fetches, not only artwork.
    ...sentryConnectSources(),
    ...(isDevEnv ? ["ws:", "wss:"] : []), // Next HMR, including LAN/HTTPS proxies.
  ].join(" ")}`,
  // Same-origin map workers; fflate uses blob workers for large XLSX exports.
  "worker-src 'self' blob:",
  "media-src 'none'",
  "object-src 'none'",
  "frame-src 'none'",
  "base-uri 'none'",
  "form-action 'self'",
  // Preserve SAMEORIGIN framing; OBS/Streamlabs browser sources load directly.
  "frame-ancestors 'self'",
  ...(!isDevEnv ? ["upgrade-insecure-requests"] : []),
].join("; ");

// Do not add COEP require-corp: Lilith no-cors avatars need upstream CORP/CORS
// cooperation. Current WASM/worker features do not require shared-memory isolation.
// No external CSP reports: overlay document URLs contain access tokens.

const config: NextConfig = {
  agentRules: false,
  poweredByHeader: false,
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
        source: "/docs/installation",
        destination: "/docs/installation/windows",
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
          { key: "Content-Security-Policy", value: contentSecurityPolicy },
          { key: "X-Content-Type-Options", value: "nosniff" },
          { key: "X-Frame-Options", value: "SAMEORIGIN" },
          { key: "X-XSS-Protection", value: "0" },
          { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
          {
            key: "Permissions-Policy",
            value:
              "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()",
          },
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
