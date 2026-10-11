# Site security headers

## Policy and limits

`next.config.ts` sends an **enforced** Content-Security-Policy on all site routes
(including trailing slashes, public assets, and worker scripts). The existing
`nosniff`, `SAMEORIGIN`, disabled legacy XSS filter, referrer, permissions, and
streaming headers are preserved. The Apple profile's Content-Type is unchanged.

`security-headers.mjs` owns the policy and the image-host list shared with Next's
`remotePatterns`. With `images.unoptimized: true`, `remotePatterns` is **not** a
browser allowlist: CSP now supplies that restriction.

This is a static-compatible baseline, **not a strict nonce/hash-based XSS policy**:

- Next's streamed RSC/hydration scripts and `@wrksz/themes/next`'s bootstrap need
  inline scripts. `script-src 'unsafe-inline'` deliberately retains them, so an
  injected inline script can still execute. `script-src-attr 'none'` separately
  blocks inline event handlers. Existing escaping/sanitization remains essential.
- React, Headless/Base UI, Recharts, Motion, and theme transitions need inline
  styles. `style-src 'unsafe-inline'` retains those attributes/style elements.
- Production allows `'wasm-unsafe-eval'`, **not** JavaScript `'unsafe-eval'`.
  Development alone permits JavaScript eval and `ws:`/`wss:` for Next HMR.
- A nonce migration would require request-time nonces, dynamic rendering of every
  affected root layout (including marketing and overlay), no shared cached HTML,
  and passing the nonce to the theme provider's own script. Adding a random or
  constant nonce in `next.config.ts` would be incorrect. Experimental SRI is not
  a drop-in fix for dynamic inline scripts.
- Production upgrades insecure subresources; development does not, preserving
  local HTTP workflows. Self-hosted Sentry must therefore be HTTPS in production.
- No reporting collector is enabled. Sentry's ordinary JS SDK does not collect
  native CSP reports. In particular, `/overlay/[overlay]` URLs contain access
  tokens: adding external `report-uri`/`report-to` globally can disclose those
  tokens through the report's document URL. A separate privacy-reviewed,
  token-safe reporting design is needed before enabling collection.

## Resource inventory

Audit baseline: `main` at `9eb7dd84`; header work continues #1023 (`49106521`).

| Feature | Browser requirement | Evidence / decision |
| --- | --- | --- |
| Next chunks, CSS, fonts, icons | `'self'` script/style/font/image | All three root layouts; Inter uses `next/font/google`, which self-hosts the build output. No Google font origin needed at runtime. |
| Lilith avatars | HTTPS image access to `imimg.lilithcdn.com`, `imv2-gl.lilithgame.com`, `plat-fau-global.lilithgame.com`, `static-gl.lilithgame.com` | Next config history, `GameAvatar`, game report samples. These are four Lilith hosts, not six. |
| Site sprites, map artwork, badges | `img-src https://cdn.rokbattles.com` | Game sprites, territory structures, commanders, equipment, formations, overlay cards. |
| Territory/Baulur binary map datasets | `connect-src https://cdn.rokbattles.com` | Map data loaders fetch CDN bytes, not only images. |
| Container parser and WebGPU/WebGL map engine | `'wasm-unsafe-eval'`, self-origin module fetches | Both `generate:wasm` build scripts; container editor and map engine consumers. Ordinary WASM/WebGPU do not require cross-origin isolation. |
| Baulur navigation worker | `worker-src 'self' blob:` | Module worker created with `new URL(..., import.meta.url)`; emitted worker remains same-origin. Blob also covers Turbopack's worker bootstrap. |
| Large XLSX exports | `worker-src blob:` | `write-excel-file` calls async `fflate.zip`; ZIP members >=160KB spawn a blob worker. A small XLSX test misses this branch. |
| CSV, XLSX, container downloads | Blob anchor downloads | These are navigations/downloads, not image or fetch sources; no `blob:` connect/image permission needed. |
| Data URI artwork | `img-src data:` | Inline SVG/CSS image assets. Does not grant data scripts or frames. |
| Site APIs and overlay polling | `connect-src 'self'` | Browser uses `/proxy`; API_URL and object-storage credentials are server-side. Do not expose API/R2 origins in CSP solely for server requests. |
| Sentry errors/tracing | Exact configured DSN origin in `connect-src` | `instrumentation-client.ts` bundles the npm SDK. Handles regional/custom ingest hosts and ports; no key/path emitted. No Sentry CDN loader or Replay configured. |
| Discord login | Same-tab `/proxy/v1/auth/discord/login` navigation | OAuth exchange is server-side; no Discord script/connect/frame permission required. `form-action 'self'` allows site forms and server actions. |
| Creator OBS/Streamlabs overlay | Direct browser-source URL, self-origin polling, CDN portraits | Existing XFO SAMEORIGIN is mirrored by `frame-ancestors 'self'`. Direct browser-source navigation is not an iframe. No token URL should be pasted into public test logs. |
| Discord embed metadata | Inline JSON script | `DiscordEmbed` is static application/json metadata, not an external Discord iframe. |
| Cloudflare CDN | No additional third-party sources by itself | Live unauthenticated `/home` response on 2026-10-11 used same-origin Next scripts and `/cdn-cgi/speculation`; no external beacon script observed. This is a snapshot, not visibility into account settings. |
| External links/downloads | Ordinary top-level navigation | GitHub releases, Discord, YouTube creators, Patreon, sponsorship links, installer/profile links do not imply script/frame/connect allowlisting. |

Historical `next.config` review found the four Lilith hosts above plus the site
CDN. A sixth image entry was `cdn.discordapp.com`, removed in `fc600646` when
Discord avatar display was removed. It is not restored speculatively. No current
iframe, audio/video, service worker, application WebSocket/SSE, analytics script,
Turnstile integration, or SharedArrayBuffer/Atomics consumer was found.

An allowlist cannot guarantee every arbitrary upstream avatar URL will match:
new Lilith hosts, CDN redirects, and edge-injected integrations need verification
and a targeted policy change. Do not replace the list with `https:` or broad
`*.lilithgame.com` / `*.sentry.io` wildcards to hide violations.

## Cloudflare and cross-origin header decisions

- Plain Cloudflare CDN proxying needs no CSP exception. Same-origin `/cdn-cgi/*`
  resources are covered. If Web Analytics is enabled later, verify its actual
  script/collection endpoints before adding `static.cloudflareinsights.com` or
  `cloudflareinsights.com`. If Turnstile is added, its script and frame need
  `https://challenges.cloudflare.com`; it is not currently used. Challenge pages
  and account-configured injections still need a staging check.
- **COOP** (`Cross-Origin-Opener-Policy`) isolates browsing-context groups, not
  subresource loads. Current Discord login is same-tab, but adding `same-origin`
  changes opener/popup behavior. No change in this compatibility-focused PR;
  evaluate with real OAuth and browser-source clients before opting in.
- **COEP** (`Cross-Origin-Embedder-Policy: require-corp`) would block cross-origin
  no-cors images without their own suitable CORP response or a CORS-enabled image
  request. An `Access-Control-Allow-Origin: *` response alone is insufficient for
  the current plain avatar `<img>` requests. We do not control all Lilith CDNs.
  `credentialless` also changes credential behavior and browser support; neither
  is required for the current non-shared-memory WASM/worker implementation.
- **CORP** governs who may embed this response; it does not permit outgoing Lilith
  images. A blanket `same-origin` asset policy could break external consumers.
  Existing framing protections are retained; no blanket CORP is added.
- **Document-Isolation-Policy** is a newer isolation mechanism, with limited
  browser support and similar cross-origin resource requirements. It is not a
  compatibility fix or needed for current WASM. No new isolation header is added.
- **HSTS** is a deployment/TLS-domain commitment, especially with subdomains and
  preload. No origin-wide/subdomain policy is introduced without confirming edge
  configuration and HTTPS readiness across those domains.

## Repeatable checks and rollout gate

Run `pnpm -F @rokbattles/site test:security` (also in CI). It covers production vs
HMR, exact image/connect origins, WASM/worker permissions, local fonts/framing,
DSN normalization/rejection, and restrictive fallback directives. Then run Biome,
`generate:wasm`, `sync:legal`, `generate:datasets`, and the production site build.
Inspect `next start` responses, not only the development server: Next compiles
headers at build time, as it does `NEXT_PUBLIC_SENTRY_DSN` in client code. Rebuild
when changing that variable, and verify the final edge-delivered header.

Before merge/deployment, verify in a production-mode browser or staging with CSP
violation logging (redact overlay paths):

- Hard-load and navigate across marketing/docs, reports, loot, rankings, account,
  Territory Lab, and Baulur; check hydration, theme changes, local fonts, and
  actual images from all configured hosts (including redirect destinations).
- Parse/download a real container; initialize WebGPU and WebGL fallback; load CDN
  map bytes; run Baulur navigation in its module worker.
- Export CSV, container, small XLSX, and an XLSX whose sheet XML exceeds 160KB;
  verify fflate's worker starts and the downloaded workbook is usable.
- Exercise login/logout and OAuth return, profile/installer downloads, server
  actions, and trusted rich content; no production JavaScript eval permission.
- Load a **test-only** overlay URL directly in OBS and Streamlabs at its configured
  viewport. Confirm transparent rendering, commander images/fonts, settings,
  test mode, and live polling/updates. Also verify same-origin preview works and
  unrelated-origin framing stays blocked. Native clients remain a separate check
  from Chromium page testing; never use a real token in committed fixtures.
- Confirm Sentry delivery using the actual build's configured DSN without
  publishing credentials or sending fake production events unintentionally.
- Check the Cloudflare-served response and any enabled challenge/analytics feature.
  CSP only restricts requests; it cannot repair missing CORS, bad CDN content
  types, access failures, or third-party outages.

Unit tests establish policy construction, not end-to-end feature compatibility.
The PR validation notes distinguish local fixtures, real resource checks, CI,
and untested production/auth/native-client behavior. Keep this PR in draft until
remaining environment-specific checks are completed; no deployment is implied.

## Primary documentation

- [Next.js CSP](https://nextjs.org/docs/app/guides/content-security-policy)
  (also read the installed Next 16.3.8 `dist/docs` guide).
- [Next.js response headers](https://nextjs.org/docs/app/api-reference/config/next-config-js/headers).
- [Sentry CSP requirements](https://www.sentry.help/en/articles/13965155-how-do-i-configure-my-content-security-policy-csp-to-allow-sentry).
- [Sentry Next.js CSP reporting](https://docs.sentry.io/platforms/javascript/guides/nextjs/security-policy-reporting/).
- [Sentry Replay CSP](https://docs.sentry.io/platforms/javascript/session-replay/#content-security-policy-csp).
- [Cloudflare CSP integration](https://developers.cloudflare.com/fundamentals/reference/policies-compliances/content-security-policies/).
- [Cloudflare JavaScript Detections and CSP](https://developers.cloudflare.com/cloudflare-challenges/challenge-types/javascript-detections/#if-you-have-a-content-security-policy-csp).
- [HTML cross-origin policies](https://html.spec.whatwg.org/multipage/browsers.html#cross-origin-opener-policies).
- [Document Isolation Policy proposal](https://wicg.github.io/document-isolation-policy/).
