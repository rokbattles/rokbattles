import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { parseReportType } from "@/lib/report-favorites";

export async function GET(req: NextRequest) {
  const authResult = await requireAuthContext();
  if (!authResult.ok) {
    return authResult.response;
  }

  const { db, user } = authResult.context;
  const searchParams = req.nextUrl.searchParams;

  const reportType = parseReportType(searchParams.get("reportType"));
  if (!reportType) {
    return NextResponse.json({ error: "Invalid reportType" }, { status: 400 });
  }

  const cursor = searchParams.get("cursor");
  let cursorDate: Date | null = null;
  if (cursor) {
    const cursorValue = Number(cursor);
    if (!Number.isFinite(cursorValue)) {
      return NextResponse.json({ error: "Invalid cursor" }, { status: 400 });
    }
    cursorDate = new Date(cursorValue);
  }

  const matchStage: Document = {
    discordId: user.discordId,
    reportType,
  };

  if (cursorDate) {
    matchStage.createdAt = { $lt: cursorDate };
  }

  const aggregationPipeline: Document[] = [
    { $match: matchStage },
    { $sort: { createdAt: -1, _id: -1 } },
    { $limit: 101 },
  ];

  if (reportType === "battle") {
    aggregationPipeline.push(
      {
        $lookup: {
          from: "battleReports",
          let: { ph: "$parentHash" },
          pipeline: [
            {
              $match: {
                $expr: { $eq: ["$metadata.parentHash", "$$ph"] },
                "report.enemy.player_id": { $nin: [-2, 0] },
              },
            },
            {
              $group: {
                _id: "$metadata.parentHash",
                count: { $sum: 1 },
                firstStart: { $min: "$report.metadata.start_date" },
                lastEnd: { $max: "$report.metadata.end_date" },
              },
            },
          ],
          as: "summary",
        },
      },
      { $unwind: "$summary" },
      {
        $lookup: {
          from: "battleReports",
          let: { ph: "$parentHash", fs: "$summary.firstStart" },
          pipeline: [
            {
              $match: {
                $expr: {
                  $and: [
                    { $eq: ["$metadata.parentHash", "$$ph"] },
                    { $eq: ["$report.metadata.start_date", "$$fs"] },
                  ],
                },
                "report.enemy.player_id": { $nin: [-2, 0] },
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
      { $unwind: "$firstDoc" }
    );
  }

  const documents = await db
    .collection("reportFavorites")
    .aggregate(aggregationPipeline, { allowDiskUse: true })
    .toArray();

  const hasMore = documents.length > 100;
  const finalDocuments = hasMore ? documents.slice(0, 100) : documents;
  const lastFavorite = finalDocuments[finalDocuments.length - 1];
  const lastCreatedAt = lastFavorite?.createdAt instanceof Date ? lastFavorite.createdAt : null;
  const nextCursor =
    reportType === "battle" && hasMore && lastCreatedAt
      ? lastCreatedAt.getTime().toString()
      : undefined;

  const items =
    reportType === "battle"
      ? finalDocuments.map((doc) => ({
          parentHash: doc.parentHash,
          count: doc.summary?.count ?? 0,
          timespan: {
            firstStart: doc.summary?.firstStart ?? 0,
            lastEnd: doc.summary?.lastEnd ?? 0,
          },
          entry: {
            hash: doc.firstDoc?.entryHash ?? "",
            startDate: Number(doc.firstDoc?.startDate) || 0,
            selfCommanderId: Number(doc.firstDoc?.selfCommanderId) || 0,
            selfSecondaryCommanderId: Number(doc.firstDoc?.selfSecondaryCommanderId) || 0,
            enemyCommanderId: Number(doc.firstDoc?.enemyCommanderId) || 0,
            enemySecondaryCommanderId: Number(doc.firstDoc?.enemySecondaryCommanderId) || 0,
          },
        }))
      : [];

  return NextResponse.json(
    {
      items,
      count: items.length,
      cursor: nextCursor,
    },
    {
      headers: {
        "Cache-Control": "no-store",
      },
    }
  );
}
