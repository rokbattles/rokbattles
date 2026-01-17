import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { parseGovernorId } from "@/lib/governor";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

type KvkAggregateDocument = {
  _id?: unknown;
  count?: unknown;
};

type KvkSummary = {
  serverId: number;
  reportCount: number;
};

function parseNumeric(value: unknown): number | null {
  if (value == null) {
    return null;
  }

  const numeric = typeof value === "bigint" ? Number(value) : Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}

export async function GET(
  _req: NextRequest,
  ctx: RouteContext<"/api/v2/governor/[governorId]/kvks">
) {
  const { governorId: governorParam } = await ctx.params;
  const governorId = parseGovernorId(governorParam);
  if (governorId == null) {
    return NextResponse.json({ error: "Invalid governorId" }, { status: 400 });
  }

  const authResult = await requireAuthContext();
  if (!authResult.ok) {
    return authResult.response;
  }

  const { db, user } = authResult.context;

  const claim = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .findOne({ discordId: user.discordId, governorId });
  if (!claim) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const matchStage: Document = {
    $and: [
      { "report.metadata.is_kvk": 1 },
      { "report.metadata.email_role": { $ne: "dungeon" } },
      { "report.metadata.server_id": { $exists: true, $ne: null } },
      {
        $or: [
          {
            "report.self.player_id": governorId,
            "report.enemy.player_id": { $nin: [-2, 0] },
          },
          {
            "report.enemy.player_id": governorId,
            "report.self.player_id": { $nin: [-2, 0] },
          },
        ],
      },
    ],
  };

  const aggregationPipeline: Document[] = [
    { $match: matchStage },
    {
      $project: {
        serverId: "$report.metadata.server_id",
        parentHash: "$metadata.parentHash",
      },
    },
    {
      $group: {
        _id: {
          serverId: "$serverId",
          parentHash: "$parentHash",
        },
      },
    },
    {
      $group: {
        _id: "$_id.serverId",
        count: { $sum: 1 },
      },
    },
    { $sort: { count: -1, _id: 1 } },
  ];

  try {
    const documents = await db
      .collection<KvkAggregateDocument>("battleReports")
      .aggregate(aggregationPipeline, { allowDiskUse: true })
      .toArray();

    const items: KvkSummary[] = [];

    for (const doc of documents) {
      const serverId = parseNumeric(doc._id);
      if (serverId == null) {
        continue;
      }

      items.push({
        serverId,
        reportCount: parseNumeric(doc.count) ?? 0,
      });
    }

    return NextResponse.json(
      {
        items,
        count: items.length,
      },
      {
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  } catch (error) {
    console.error("Failed to load kvk summary", error);
    return NextResponse.json(
      { error: "Failed to load kvk summary" },
      {
        status: 500,
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  }
}
