import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import client from "@/lib/mongo";

export async function GET(req: NextRequest) {
  const searchParams = req.nextUrl.searchParams;

  const cursor = searchParams.get("cursor");
  const type = searchParams.get("type");
  const playerId = searchParams.get("pid");
  const senderPrimaryCommanderId = searchParams.get("spc");
  const senderSecondaryCommanderId = searchParams.get("ssc");
  const opponentPrimaryCommanderId = searchParams.get("opc");
  const opponentSecondaryCommanderId = searchParams.get("osc");
  const rallySideParam = searchParams.get("rs");
  const garrisonSideParam = searchParams.get("gs");
  const garrisonBuildingParam = searchParams.get("gb");

  type ReportsFilterSide = "none" | "sender" | "opponent" | "both";
  type ReportsGarrisonBuildingType = "flag" | "fortress" | "other";

  const parseSide = (value: string | null): ReportsFilterSide | undefined => {
    if (!value) return undefined;
    if (value === "none" || value === "sender" || value === "opponent" || value === "both") {
      return value;
    }
    return undefined;
  };

  const parseGarrisonBuilding = (value: string | null): ReportsGarrisonBuildingType | undefined => {
    if (!value) return undefined;
    if (value === "flag" || value === "fortress" || value === "other") {
      return value;
    }
    return undefined;
  };

  let parsedType: string | undefined;
  if (type) {
    if (["kvk", "ark", "home"].includes(type)) {
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

  let parsedSenderPrimaryCommanderId: number | undefined;
  if (senderPrimaryCommanderId) {
    const n = Number(senderPrimaryCommanderId);
    if (Number.isFinite(n)) {
      parsedSenderPrimaryCommanderId = n;
    } else {
      return NextResponse.json({ error: "Invalid sender primary commander id" }, { status: 400 });
    }
  }

  let parsedSenderSecondaryCommanderId: number | undefined;
  if (senderSecondaryCommanderId) {
    const n = Number(senderSecondaryCommanderId);
    if (Number.isFinite(n)) {
      parsedSenderSecondaryCommanderId = n;
    } else {
      return NextResponse.json({ error: "Invalid sender secondary commander id" }, { status: 400 });
    }
  }

  let parsedOpponentPrimaryCommanderId: number | undefined;
  if (opponentPrimaryCommanderId) {
    const n = Number(opponentPrimaryCommanderId);
    if (Number.isFinite(n)) {
      parsedOpponentPrimaryCommanderId = n;
    } else {
      return NextResponse.json({ error: "Invalid opponent primary commander id" }, { status: 400 });
    }
  }

  let parsedOpponentSecondaryCommanderId: number | undefined;
  if (opponentSecondaryCommanderId) {
    const n = Number(opponentSecondaryCommanderId);
    if (Number.isFinite(n)) {
      parsedOpponentSecondaryCommanderId = n;
    } else {
      return NextResponse.json(
        { error: "Invalid opponent secondary commander id" },
        { status: 400 }
      );
    }
  }

  const parsedRallySide = parseSide(rallySideParam) ?? "none";
  if (rallySideParam && !parseSide(rallySideParam)) {
    return NextResponse.json({ error: "Invalid rally side" }, { status: 400 });
  }

  const parsedGarrisonSide = parseSide(garrisonSideParam) ?? "none";
  if (garrisonSideParam && !parseSide(garrisonSideParam)) {
    return NextResponse.json({ error: "Invalid garrison side" }, { status: 400 });
  }

  const parsedGarrisonBuilding = parseGarrisonBuilding(garrisonBuildingParam);
  if (garrisonBuildingParam && !parsedGarrisonBuilding) {
    return NextResponse.json({ error: "Invalid garrison building" }, { status: 400 });
  }

  const sideOverlaps =
    ((parsedRallySide === "sender" || parsedRallySide === "both") &&
      (parsedGarrisonSide === "sender" || parsedGarrisonSide === "both")) ||
    ((parsedRallySide === "opponent" || parsedRallySide === "both") &&
      (parsedGarrisonSide === "opponent" || parsedGarrisonSide === "both"));
  if (sideOverlaps) {
    return NextResponse.json(
      { error: "Rally and garrison cannot overlap on the same side" },
      { status: 400 }
    );
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

  if (parsedSenderPrimaryCommanderId !== undefined) {
    matchPipeline.push({
      "report.self.primary_commander.id": parsedSenderPrimaryCommanderId,
    });
  }

  if (parsedSenderSecondaryCommanderId !== undefined) {
    matchPipeline.push({
      "report.self.secondary_commander.id": parsedSenderSecondaryCommanderId,
    });
  }

  if (parsedOpponentPrimaryCommanderId !== undefined) {
    matchPipeline.push({
      "report.enemy.primary_commander.id": parsedOpponentPrimaryCommanderId,
    });
  }

  if (parsedOpponentSecondaryCommanderId !== undefined) {
    matchPipeline.push({
      "report.enemy.secondary_commander.id": parsedOpponentSecondaryCommanderId,
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

    if (parsedType === "home") {
      matchPipeline.push({ "report.metadata.is_kvk": 0 });
      matchPipeline.push({ "report.metadata.email_role": { $ne: "dungeon" } });
    }
  }

  const rallyConditions: Document[] = [];
  if (parsedRallySide === "sender" || parsedRallySide === "both") {
    rallyConditions.push({ "report.self.is_rally": { $in: [1, true] } });
  }
  if (parsedRallySide === "opponent" || parsedRallySide === "both") {
    rallyConditions.push({ "report.enemy.is_rally": { $in: [1, true] } });
  }
  if (rallyConditions.length === 1) {
    matchPipeline.push(rallyConditions[0]);
  } else if (rallyConditions.length > 1) {
    matchPipeline.push({ $or: rallyConditions });
  }

  const buildGarrisonCondition = (path: string) => {
    if (parsedGarrisonBuilding === "flag") {
      return { [path]: 1 };
    }
    if (parsedGarrisonBuilding === "fortress") {
      return { [path]: 3 };
    }
    if (parsedGarrisonBuilding === "other") {
      return { [path]: { $exists: true, $nin: [1, 3, null] } };
    }
    return { [path]: { $exists: true, $ne: null } };
  };

  const garrisonConditions: Document[] = [];
  if (parsedGarrisonSide === "sender" || parsedGarrisonSide === "both") {
    garrisonConditions.push(buildGarrisonCondition("report.self.alliance_building"));
  }
  if (parsedGarrisonSide === "opponent" || parsedGarrisonSide === "both") {
    garrisonConditions.push(buildGarrisonCondition("report.enemy.alliance_building"));
  }
  if (garrisonConditions.length === 1) {
    matchPipeline.push(garrisonConditions[0]);
  } else if (garrisonConditions.length > 1) {
    matchPipeline.push({ $or: garrisonConditions });
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
