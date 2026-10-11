import assert from "node:assert/strict";
import { test } from "node:test";
import { contentSecurityPolicy, imageHosts } from "../security-headers.mjs";

function policy(options) {
  return Object.fromEntries(
    contentSecurityPolicy(options)
      .split("; ")
      .map((directive) => {
        const [name, ...sources] = directive.split(" ");
        return [name, sources];
      })
  );
}

test("production enables WASM without enabling JavaScript eval", () => {
  const csp = policy();
  assert.deepEqual(csp["script-src"], ["'self'", "'unsafe-inline'", "'wasm-unsafe-eval'"]);
  assert.deepEqual(csp["script-src-attr"], ["'none'"]);
  assert.deepEqual(csp["upgrade-insecure-requests"], []);
  assert.ok(!contentSecurityPolicy().includes("'unsafe-eval'"));
});

test("retains Next/theme inline bootstrap and React inline styles without fake static nonces", () => {
  const csp = policy();
  assert.ok(csp["script-src"].includes("'unsafe-inline'"));
  assert.deepEqual(csp["style-src"], ["'self'", "'unsafe-inline'"]);
  assert.ok(!contentSecurityPolicy().includes("nonce-"));
});

test("permits all four historical Lilith image hosts and site artwork, not arbitrary HTTPS", () => {
  assert.deepEqual(imageHosts, [
    "imimg.lilithcdn.com",
    "imv2-gl.lilithgame.com",
    "plat-fau-global.lilithgame.com",
    "static-gl.lilithgame.com",
    "cdn.rokbattles.com",
  ]);
  assert.deepEqual(policy()["img-src"], [
    "'self'",
    "data:",
    ...imageHosts.map((host) => `https://${host}`),
  ]);
});

test("allows same-origin proxy/polling and CDN map fetches, but no unrelated APIs", () => {
  assert.deepEqual(policy()["connect-src"], ["'self'", "https://cdn.rokbattles.com"]);
});

test("allows emitted module workers and fflate's large-XLSX blob worker", () => {
  assert.deepEqual(policy()["worker-src"], ["'self'", "blob:"]);
});

test("allows self-hosted next/font and preserves same-origin framing for OBS URLs", () => {
  const csp = policy();
  assert.deepEqual(csp["font-src"], ["'self'"]);
  assert.deepEqual(csp["frame-ancestors"], ["'self'"]);
  assert.deepEqual(csp["form-action"], ["'self'"]);
});

test("blocks unused embedding, media, plugins, and base-tag injection", () => {
  const csp = policy();
  for (const name of ["frame-src", "media-src", "object-src", "base-uri"]) {
    assert.deepEqual(csp[name], ["'none'"], name);
  }
  assert.deepEqual(csp["default-src"], ["'self'"]);
});

test("derives exact regional Sentry ingest origin without leaking DSN key or project", () => {
  const csp = policy({ sentryDsn: "https://public-key@o123.ingest.us.sentry.io/456" });
  assert.deepEqual(csp["connect-src"], [
    "'self'",
    "https://cdn.rokbattles.com",
    "https://o123.ingest.us.sentry.io",
  ]);
  assert.ok(!JSON.stringify(csp).includes("public-key"));
  assert.ok(!JSON.stringify(csp).includes("456"));
});

test("supports self-hosted Sentry with a path prefix and port, and deduplicates origins", () => {
  assert.deepEqual(
    policy({ sentryDsn: "https://key@errors.example.com:8443/prefix/7" })["connect-src"],
    ["'self'", "https://cdn.rokbattles.com", "https://errors.example.com:8443"]
  );
  assert.deepEqual(
    policy({ sentryDsn: "https://key@cdn.rokbattles.com/7" })["connect-src"],
    policy()["connect-src"]
  );
});

test("empty Sentry config adds no origin; malformed config fails instead of widening policy", () => {
  assert.deepEqual(policy({ sentryDsn: " " }), policy());
  for (const sentryDsn of ["invalid", "javascript:alert(1)", "data:text/plain,test"]) {
    assert.throws(() => contentSecurityPolicy({ sentryDsn }), /absolute HTTP\(S\) URL/);
  }
});

test("development alone permits HMR/eval and does not upgrade local HTTP", () => {
  const dev = policy({ development: true });
  assert.ok(dev["script-src"].includes("'unsafe-eval'"));
  assert.deepEqual(dev["connect-src"], ["'self'", "https://cdn.rokbattles.com", "ws:", "wss:"]);
  assert.equal(dev["upgrade-insecure-requests"], undefined);
  assert.ok(!policy()["connect-src"].some((source) => source.startsWith("ws")));
});

test("serializes a single-line header with unique directives", () => {
  const header = contentSecurityPolicy();
  assert.ok(!/[\r\n]/.test(header));
  assert.equal(header.split("; ").length, Object.keys(policy()).length);
});
