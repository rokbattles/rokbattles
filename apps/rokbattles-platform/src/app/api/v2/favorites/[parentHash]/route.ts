import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { parseReportType } from "@/lib/report-favorites";
import type { ReportFavoriteDocument } from "@/lib/types/favorite";

function normalizeParentHash(value: string | null | undefined) {
  const trimmed = value?.trim() ?? "";
  return trimmed.length > 0 ? trimmed : null;
}

export async function GET(
  req: NextRequest,
  ctx: RouteContext<"/api/v2/favorites/[parentHash]">
) {
  const authResult = await requireAuthContext();
  if (!authResult.ok) {
    return authResult.response;
  }

  const { parentHash } = await ctx.params;
  const normalizedHash = normalizeParentHash(parentHash);
  if (!normalizedHash) {
    return NextResponse.json({ error: "Missing report hash" }, { status: 400 });
  }

  const reportType = parseReportType(
    req.nextUrl.searchParams.get("reportType")
  );
  if (!reportType) {
    return NextResponse.json({ error: "Invalid reportType" }, { status: 400 });
  }

  const { db, user } = authResult.context;

  const existingFavorite = await db
    .collection<ReportFavoriteDocument>("reportFavorites")
    .findOne(
      {
        discordId: user.discordId,
        reportType,
        parentHash: normalizedHash,
      },
      { projection: { _id: 1 } }
    );

  return NextResponse.json(
    { favorited: Boolean(existingFavorite) },
    {
      headers: {
        "Cache-Control": "no-store",
      },
    }
  );
}

export async function POST(
  req: NextRequest,
  ctx: RouteContext<"/api/v2/favorites/[parentHash]">
) {
  const authResult = await requireAuthContext();
  if (!authResult.ok) {
    return authResult.response;
  }

  const { parentHash } = await ctx.params;
  const normalizedHash = normalizeParentHash(parentHash);
  if (!normalizedHash) {
    return NextResponse.json({ error: "Missing report hash" }, { status: 400 });
  }

  const reportType = parseReportType(
    req.nextUrl.searchParams.get("reportType")
  );
  if (!reportType) {
    return NextResponse.json({ error: "Invalid reportType" }, { status: 400 });
  }

  const { db, user } = authResult.context;
  const now = new Date();

  await db.collection<ReportFavoriteDocument>("reportFavorites").updateOne(
    {
      discordId: user.discordId,
      reportType,
      parentHash: normalizedHash,
    },
    {
      $setOnInsert: {
        discordId: user.discordId,
        reportType,
        parentHash: normalizedHash,
        createdAt: now,
      },
    },
    { upsert: true }
  );

  return NextResponse.json(
    { favorited: true },
    {
      status: 201,
      headers: {
        "Cache-Control": "no-store",
      },
    }
  );
}

export async function DELETE(
  req: NextRequest,
  ctx: RouteContext<"/api/v2/favorites/[parentHash]">
) {
  const authResult = await requireAuthContext();
  if (!authResult.ok) {
    return authResult.response;
  }

  const { parentHash } = await ctx.params;
  const normalizedHash = normalizeParentHash(parentHash);
  if (!normalizedHash) {
    return NextResponse.json({ error: "Missing report hash" }, { status: 400 });
  }

  const reportType = parseReportType(
    req.nextUrl.searchParams.get("reportType")
  );
  if (!reportType) {
    return NextResponse.json({ error: "Invalid reportType" }, { status: 400 });
  }

  const { db, user } = authResult.context;

  await db.collection<ReportFavoriteDocument>("reportFavorites").deleteOne({
    discordId: user.discordId,
    reportType,
    parentHash: normalizedHash,
  });

  return NextResponse.json(
    { favorited: false },
    {
      headers: {
        "Cache-Control": "no-store",
      },
    }
  );
}
