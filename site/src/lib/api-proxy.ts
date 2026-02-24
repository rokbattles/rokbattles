import "server-only";

import { type NextRequest, NextResponse } from "next/server";

const DEFAULT_ROKBATTLES_API_URL = "http://127.0.0.1:8001";
const BODYLESS_METHODS = new Set(["GET", "HEAD"]);

function getApiBaseUrl() {
  const configured = process.env.ROKBATTLES_API_URL ?? DEFAULT_ROKBATTLES_API_URL;
  return configured.endsWith("/") ? configured.slice(0, -1) : configured;
}

function buildForwardHeaders(req: NextRequest) {
  const headers = new Headers();

  const cookie = req.headers.get("cookie");
  if (cookie) {
    headers.set("cookie", cookie);
  }

  const userAgent = req.headers.get("user-agent");
  if (userAgent) {
    headers.set("user-agent", userAgent);
  }

  const accept = req.headers.get("accept");
  if (accept) {
    headers.set("accept", accept);
  }

  const contentType = req.headers.get("content-type");
  if (contentType) {
    headers.set("content-type", contentType);
  }

  return headers;
}

function buildResponseHeaders(upstream: Response) {
  const responseHeaders = new Headers();
  const contentType = upstream.headers.get("content-type");
  if (contentType) {
    responseHeaders.set("content-type", contentType);
  }
  responseHeaders.set("cache-control", "no-store");
  return responseHeaders;
}

export async function proxyApiRequest(req: NextRequest, endpointPath: string) {
  const method = req.method.toUpperCase();
  const upstreamUrl = `${getApiBaseUrl()}${endpointPath}${req.nextUrl.search}`;
  const body = BODYLESS_METHODS.has(method) ? undefined : await req.arrayBuffer();

  try {
    const upstream = await fetch(upstreamUrl, {
      method,
      headers: buildForwardHeaders(req),
      body,
      cache: "no-store",
    });

    return new NextResponse(await upstream.arrayBuffer(), {
      status: upstream.status,
      headers: buildResponseHeaders(upstream),
    });
  } catch (error) {
    console.error("Failed to proxy request to API", error);
    return NextResponse.json({ error: "upstream-unavailable" }, { status: 502 });
  }
}
