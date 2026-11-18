async function readNonce(request: Request): Promise<string | null> {
  const reader = request.body?.getReader();
  if (!reader) return null;

  const bytes = new Uint8Array(32);
  let length = 0;

  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;

      if (length + value.length > bytes.length) {
        void reader.cancel().catch(() => {});
        return null;
      }

      bytes.set(value, length);
      length += value.length;
    }
  } finally {
    reader.releaseLock();
  }

  if (length !== bytes.length) return null;

  const nonce = new TextDecoder().decode(bytes);
  return /^[0-9a-f]{32}$/.test(nonce) ? nonce : null;
}

export async function POST(
  request: Request,
  { params }: RouteContext<"/api/dns-check/[operation]">
): Promise<Response> {
  const reply = (status: number) =>
    new Response(null, { status, headers: { "Cache-Control": "no-store" } });

  const { operation } = await params;
  if (operation !== "start" && operation !== "result") return reply(404);

  const secret = process.env.DNS_PROBE_SECRET;
  if (!secret) return reply(503);

  if (request.headers.get("content-type") !== "application/octet-stream") return reply(415);

  try {
    const nonce = await readNonce(request);
    if (!nonce) return reply(400);

    const response = await fetch(`https://dns.rokbattles.com/dns-probe/${operation}`, {
      method: "POST",
      body: nonce,
      cache: "no-store",
      headers: {
        Authorization: `Bearer ${secret}`,
        "Content-Type": "application/octet-stream",
      },
      redirect: "error",
      signal: AbortSignal.any([request.signal, AbortSignal.timeout(11_000)]),
    });

    if (response.status === 204) return reply(204);
    if (operation === "result" && response.status === 404) return reply(404);
    return reply(503);
  } catch {
    return reply(503);
  }
}
