import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import client from "@/lib/mongo";

export async function GET(req: NextRequest) {
  const searchParams = req.nextUrl.searchParams;

  const cursor = searchParams.get("cursor");
  const type = searchParams.get("type");
  const playerId = searchParams.get("playerId");
  const primaryCommanderId = searchParams.get("primaryCommanderId");
  const secondaryCommanderId = searchParams.get("secondaryCommanderId");
  const rallyOnly = searchParams.get("rallyOnly");

  let parsedType: string | undefined;
  if (type) {
    if (["kvk", "ark"].includes(type)) {
      parsedType = type;
    } else {
      return NextResponse.json({ error: "Invalid type" }, { status: 400 });
    }
  }

  let parsedPlayerId: number | undefined;
  if (playerId) {
    const n = Number(playerId);
    if (Number.isFinite(n)) {
      parsedPlayerId = n;
    } else {
      return NextResponse.json({ error: "Invalid governor id" }, { status: 400 });
    }
  }

  let parsedPrimaryCommanderId: number | undefined;
  if (primaryCommanderId) {
    const n = Number(primaryCommanderId);
    if (Number.isFinite(n)) {
      parsedPrimaryCommanderId = n;
    } else {
      return NextResponse.json({ error: "Invalid primary commander id" }, { status: 400 });
    }
  }

  let parsedSecondaryCommanderId: number | undefined;
  if (secondaryCommanderId) {
    const n = Number(secondaryCommanderId);
    if (Number.isFinite(n)) {
      parsedSecondaryCommanderId = n;
    } else {
      return NextResponse.json({ error: "Invalid secondary commander id" }, { status: 400 });
    }
  }

  const mongo = await client.connect();
  const db = mongo.db();

  const matchPipeline: Document[] = [
    // Filter out npc & empty structures
    // TODO consider $gt 0 to see if its more performant
    { "report.enemy.player_id": { $nin: [-2, 0] } },
  ];

  if (parsedPlayerId !== undefined) {
    matchPipeline.push({
      $or: [
        { "report.self.player_id": parsedPlayerId },
        { "report.enemy.player_id": parsedPlayerId },
      ],
    });
  }

  if (parsedPrimaryCommanderId !== undefined) {
    matchPipeline.push({
      $or: [
        { "report.self.primary_commander.id": parsedPrimaryCommanderId },
        { "report.enemy.primary_commander.id": parsedPrimaryCommanderId },
      ],
    });
  }

  if (parsedSecondaryCommanderId !== undefined) {
    matchPipeline.push({
      $or: [
        { "report.self.secondary_commander.id": parsedSecondaryCommanderId },
        { "report.enemy.secondary_commander.id": parsedSecondaryCommanderId },
      ],
    });
  }

  if (parsedType) {
    if (parsedType === "kvk") {
      matchPipeline.push({ "report.metadata.is_kvk": 1 });
      matchPipeline.push({ "report.metadata.email_role": { $ne: "dungeon" } });
    }

    if (parsedType === "ark") {
      matchPipeline.push({ "report.metadata.email_role": "dungeon" });
    }
  }

  if (rallyOnly === "1") {
    matchPipeline.push({
      $or: [{ "report.self.is_rally": 1 }, { "report.enemy.is_rally": 1 }],
    });
  }

  const finalMatchPipeline =
    matchPipeline.length === 1 ? matchPipeline[0] : { $and: matchPipeline };

  const aggregationPipeline: Document[] = [
    { $match: finalMatchPipeline },
    {
      $group: {
        _id: "$metadata.parentHash",
        count: { $sum: 1 },
        firstStart: { $min: "$report.metadata.start_date" },
        latestMailTime: { $max: "$report.metadata.email_time" },
        lastEnd: { $max: "$report.metadata.end_date" },
      },
    },
    ...(cursor
      ? [
          {
            $match: {
              $expr: { $lt: ["$latestMailTime", { $toLong: cursor }] },
            },
          },
        ]
      : []),
    { $sort: { latestMailTime: -1 } },
    { $limit: 101 },
    {
      $lookup: {
        from: "battleReports",
        let: { ph: "$_id", fs: "$firstStart" },
        pipeline: [
          {
            $match: {
              $expr: {
                $and: [
                  { $eq: ["$metadata.parentHash", "$$ph"] },
                  { $eq: ["$report.metadata.start_date", "$$fs"] },
                ],
              },
            },
          },
          { $sort: { "metadata.hash": 1 } },
          {
            $project: {
              entryHash: "$metadata.hash",
              startDate: "$report.metadata.start_date",
              selfCommanderId: "$report.self.primary_commander.id",
              selfSecondaryCommanderId: { $ifNull: ["$report.self.secondary_commander.id", 0] },
              enemyCommanderId: "$report.enemy.primary_commander.id",
              enemySecondaryCommanderId: { $ifNull: ["$report.enemy.secondary_commander.id", 0] },
            },
          },
          { $limit: 1 },
        ],
        as: "firstDoc",
      },
    },
    { $unwind: "$firstDoc" },
  ];

  const documents = await db
    .collection("battleReports")
    .aggregate(aggregationPipeline, { allowDiskUse: true })
    .toArray();
  const hasMore = documents.length > 100;
  const finalDocuments = hasMore ? documents.slice(0, 100) : documents;
  const finalCursor: string | undefined = hasMore
    ? finalDocuments[finalDocuments.length - 1].latestMailTime.toString()
    : undefined;

  const items = finalDocuments.map((d) => ({
    parentHash: d._id,
    count: d.count,
    timespan: { firstStart: d.firstStart, lastEnd: d.lastEnd },
    entry: {
      hash: d.firstDoc.entryHash,
      startDate: Number(d.firstDoc.startDate) || 0,
      selfCommanderId: Number(d.firstDoc.selfCommanderId) || 0,
      selfSecondaryCommanderId: Number(d.firstDoc.selfSecondaryCommanderId) || 0,
      enemyCommanderId: Number(d.firstDoc.enemyCommanderId) || 0,
      enemySecondaryCommanderId: Number(d.firstDoc.enemySecondaryCommanderId) || 0,
    },
  }));

  return NextResponse.json(
    {
      items,
      count: items.length,
      cursor: finalCursor,
    },
    {
      headers: {
        "Cache-Control": "no-store",
      },
    }
  );
}
