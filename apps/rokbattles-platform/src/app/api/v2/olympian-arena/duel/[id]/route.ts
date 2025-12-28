import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import client from "@/lib/mongo";

type DuelReportEntry = {
  report: Record<string, unknown>;
};

function toPlainObject(source: unknown): Record<string, unknown> {
  if (!source || typeof source !== "object") {
    return {};
  }

  const serialized = JSON.stringify(source, (_, value) => {
    if (
      value &&
      typeof value === "object" &&
      "$numberLong" in (value as Record<string, unknown>) &&
      typeof (value as Record<string, unknown>).$numberLong === "string"
    ) {
      const numericValue = Number((value as { $numberLong: string }).$numberLong);
      return Number.isFinite(numericValue)
        ? numericValue
        : (value as { $numberLong: string }).$numberLong;
    }
    return value;
  });

  return JSON.parse(serialized) as Record<string, unknown>;
}

export async function GET(
  _req: NextRequest,
  ctx: RouteContext<"/api/v2/olympian-arena/duel/[id]">
) {
  const { id } = await ctx.params;
  if (!id) {
    return NextResponse.json({ error: "Missing duel id" }, { status: 400 });
  }

  const parsedId = Number(id);
  if (!Number.isFinite(parsedId) || parsedId <= 0) {
    return NextResponse.json({ error: "Invalid duel id" }, { status: 400 });
  }

  const duelId = Math.trunc(parsedId);

  try {
    const mongo = await client.connect();
    const db = mongo.db();

    const matchPipeline: Document = {
      "report.sender.duel_id": duelId,
    };

    const aggregationPipeline: Document[] = [
      { $match: matchPipeline },
      {
        $project: {
          _id: 0,
          report: "$report",
        },
      },
      { $sort: { "report.metadata.email_time": 1 } },
    ];

    const documents = await db
      .collection("duelbattle2Reports")
      .aggregate(aggregationPipeline, { allowDiskUse: true })
      .toArray();

    const items: DuelReportEntry[] = documents.map((doc) => ({
      report: toPlainObject(doc.report),
    }));

    return NextResponse.json(
      {
        duelId,
        items,
        count: items.length,
      },
      {
        headers: {
          "Cache-Control": "public, max-age=604800, immutable",
        },
      }
    );
  } catch (error) {
    console.error("Failed to load duel reports", error);
    return NextResponse.json(
      { error: "Failed to load duel reports" },
      {
        status: 500,
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  }
}
