import "server-only";

export function rewriteUrl(req: Request) {
  const forwardedHost = req.headers.get("x-forwarded-host");
  const forwardedProto = req.headers.get("x-forwarded-proto");

  if (forwardedHost && forwardedProto) {
    const url = new URL(req.url);
    url.host = forwardedHost;
    url.protocol = forwardedProto;
    url.port = "";

    return new Request(url, req);
  }

  return req;
}
