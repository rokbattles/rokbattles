// Keep the image optimizer and browser policy in sync. Images are currently
// unoptimized, so Next's remotePatterns alone does not restrict browser loads.
export const imageHosts = [
  "imimg.lilithcdn.com",
  "imv2-gl.lilithgame.com",
  "plat-fau-global.lilithgame.com",
  "static-gl.lilithgame.com",
  "cdn.rokbattles.com",
];

/**
 * Only include the DSN's origin, never its public key, project, query, or path.
 * This supports regional Sentry ingest and self-hosted Sentry without granting
 * every sentry.io subdomain permission to receive browser requests.
 * @param {string | undefined} dsn
 */
function sentryOrigin(dsn) {
  if (!dsn?.trim()) return [];
  let url;
  try {
    url = new URL(dsn);
  } catch {
    throw new Error("NEXT_PUBLIC_SENTRY_DSN must be an absolute HTTP(S) URL");
  }
  if (url.protocol !== "https:" && url.protocol !== "http:") {
    throw new Error("NEXT_PUBLIC_SENTRY_DSN must be an absolute HTTP(S) URL");
  }
  return [url.origin];
}

/**
 * Static-compatible CSP: see SECURITY_HEADERS.md for the resource inventory,
 * inline-script tradeoff, cross-origin policy decisions, and browser checks.
 * @param {{ development?: boolean, sentryDsn?: string }} options
 */
export function contentSecurityPolicy({ development = false, sentryDsn } = {}) {
  const directives = {
    "default-src": ["'self'"],
    // Next's RSC/hydration and the theme bootstrap contain inline scripts.
    // WASM permission does not grant JavaScript eval in production.
    "script-src": [
      "'self'",
      "'unsafe-inline'",
      "'wasm-unsafe-eval'",
      ...(development ? ["'unsafe-eval'"] : []),
    ],
    "script-src-attr": ["'none'"],
    // React, charts, motion, and UI primitives set inline style attributes.
    "style-src": ["'self'", "'unsafe-inline'"],
    "img-src": ["'self'", "data:", ...imageHosts.map((host) => `https://${host}`)],
    // next/font downloads Google fonts at build time and serves them locally.
    "font-src": ["'self'"],
    "connect-src": [
      "'self'",
      "https://cdn.rokbattles.com",
      ...sentryOrigin(sentryDsn),
      // Next's development HMR websocket can use a LAN host or HTTPS proxy.
      ...(development ? ["ws:", "wss:"] : []),
    ],
    "worker-src": ["'self'", "blob:"],
    "media-src": ["'none'"],
    "object-src": ["'none'"],
    "frame-src": ["'none'"],
    "base-uri": ["'none'"],
    "form-action": ["'self'"],
    // Matches existing X-Frame-Options; OBS loads its browser source directly.
    "frame-ancestors": ["'self'"],
    ...(!development ? { "upgrade-insecure-requests": [] } : {}),
  };
  return Object.entries(directives)
    .map(([directive, sources]) => [directive, ...new Set(sources)].join(" "))
    .join("; ");
}
