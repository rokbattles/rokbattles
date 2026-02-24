import type { NextRequest } from "next/server";
import { proxyApiRequest } from "@/lib/api-proxy";

function buildUpstreamPath(path: string[] | undefined) {
  if (!path || path.length === 0) {
    return "/";
  }

  return `/${path.map((segment) => encodeURIComponent(segment)).join("/")}`;
}

async function handleProxy(req: NextRequest, ctx: RouteContext<"/proxy/[[...path]]">) {
  const { path } = await ctx.params;
  return proxyApiRequest(req, buildUpstreamPath(path));
}

export async function GET(req: NextRequest, ctx: RouteContext<"/proxy/[[...path]]">) {
  return handleProxy(req, ctx);
}

export async function POST(req: NextRequest, ctx: RouteContext<"/proxy/[[...path]]">) {
  return handleProxy(req, ctx);
}

export async function PUT(req: NextRequest, ctx: RouteContext<"/proxy/[[...path]]">) {
  return handleProxy(req, ctx);
}

export async function PATCH(req: NextRequest, ctx: RouteContext<"/proxy/[[...path]]">) {
  return handleProxy(req, ctx);
}

export async function DELETE(req: NextRequest, ctx: RouteContext<"/proxy/[[...path]]">) {
  return handleProxy(req, ctx);
}

export async function HEAD(req: NextRequest, ctx: RouteContext<"/proxy/[[...path]]">) {
  return handleProxy(req, ctx);
}

export async function OPTIONS(req: NextRequest, ctx: RouteContext<"/proxy/[[...path]]">) {
  return handleProxy(req, ctx);
}
