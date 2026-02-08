import { NextResponse } from "next/server";
import { refreshBinds } from "@/lib/bind";
import clientPromise from "@/lib/mongo";

function isAuthorized(authHeader: string | null, cronSecret: string): boolean {
  if (!authHeader) {
    return false;
  }

  const [scheme, token] = authHeader.split(" ");
  return scheme === "Bearer" && token === cronSecret;
}

export async function POST(req: Request) {
  const cronSecret = process.env.CRON_SECRET;
  if (!cronSecret) {
    return NextResponse.json(
      { error: "CRON_SECRET is not configured" },
      { status: 500 }
    );
  }

  const authHeader = req.headers.get("authorization");
  if (!isAuthorized(authHeader, cronSecret)) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const client = await clientPromise;
  if (!client) {
    return NextResponse.json(
      { error: "MongoDB client unavailable" },
      { status: 500 }
    );
  }

  const summary = await refreshBinds(client.db());

  return NextResponse.json({
    ok: true,
    summary,
  });
}
