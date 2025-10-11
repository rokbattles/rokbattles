import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import client from "@/lib/mongo";

export async function GET(req: NextRequest) {
  const searchParams = req.nextUrl.searchParams;

  const cursor = searchParams.get("cursor");
  const type = searchParams.get("type");
  const playerId = searchParams.get("playerId");
  const commanderId = searchParams.get("commanderId");

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
      parsedPlayerId = Math.trunc(n);
    } else {
      return NextResponse.json({ error: "Invalid playerId" }, { status: 400 });
    }
  }

  let parsedCommanderId: number | undefined;
  if (commanderId) {
    const n = Number(commanderId);
    if (Number.isFinite(n)) {
      parsedCommanderId = Math.trunc(n);
    } else {
      return NextResponse.json({ error: "Invalid commanderId" }, { status: 400 });
    }
  }

  const mongo = await client.connect();
  const db = mongo.db();

  const matchPipeline: Document[] = [
    // Filter out npc & empty structures
    // TODO consider $gt 0 to see if its more performant
    { "report.enemy.player_id": { $nin: [-2, 0] } },
  ];

  if (parsedPlayerId) {
    matchPipeline.push({
      $or: [
        { "report.self.player_id": parsedPlayerId },
        { "report.enemy.player_id": parsedPlayerId },
      ],
    });
  }

  if (parsedCommanderId) {
    matchPipeline.push({
      $or: [
        { "report.self.primary_commander.id": parsedCommanderId },
        { "report.self.secondary_commander.id": parsedCommanderId },
        { "report.enemy.primary_commander.id": parsedCommanderId },
        { "report.enemy.secondary_commander.id": parsedCommanderId },
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
