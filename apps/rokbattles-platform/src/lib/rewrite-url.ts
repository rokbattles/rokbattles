export function rewriteUrl(req: Request) {
  const forwardedHost = req.headers.get("x-forwarded-host");
  const forwardedProto = req.headers.get("x-forwarded-proto");

  if (forwardedHost && forwardedProto) {
    const url = new URL(req.url);

    url.hostname = forwardedHost;
    url.protocol = forwardedProto;
    url.port = "";

    return new Request(url, req);
  }

  return req;
}
