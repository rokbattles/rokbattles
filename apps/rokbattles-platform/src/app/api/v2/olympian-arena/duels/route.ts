import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import clientPromise from "@/lib/mongo";

export async function GET(req: NextRequest) {
  const searchParams = req.nextUrl.searchParams;

  const cursor = searchParams.get("cursor");

  const mongo = await clientPromise;
  const db = mongo.db();

  const matchPipeline: Document[] = [
    { "report.sender.duel_id": { $exists: true, $ne: null } },
    { "report.metadata.email_time": { $exists: true, $ne: null } },
  ];

  const finalMatchPipeline =
    matchPipeline.length === 1 ? matchPipeline[0] : { $and: matchPipeline };

  const aggregationPipeline: Document[] = [
    { $match: finalMatchPipeline },
    {
      $group: {
        _id: "$report.sender.duel_id",
        count: { $sum: 1 },
        firstMailTime: { $min: "$report.metadata.email_time" },
        latestMailTime: { $max: "$report.metadata.email_time" },
        winStreak: {
          $sum: {
            $cond: [{ $eq: ["$report.results.win", true] }, 1, 0],
          },
        },
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
        from: "duelbattle2Reports",
        let: { duelId: "$_id", fm: "$firstMailTime" },
        pipeline: [
          {
            $match: {
              $expr: {
                $and: [
                  { $eq: ["$report.sender.duel_id", "$$duelId"] },
                  { $eq: ["$report.metadata.email_time", "$$fm"] },
                ],
              },
            },
          },
          { $sort: { "report.metadata.email_time": 1 } },
          {
            $project: {
              senderPlayerId: "$report.sender.player_id",
              senderPlayerName: "$report.sender.player_name",
              senderKingdom: "$report.sender.kingdom",
              senderAlliance: "$report.sender.alliance",
              senderDuelId: "$report.sender.duel_id",
              senderAvatarUrl: "$report.sender.avatar_url",
              senderFrameUrl: "$report.sender.frame_url",
              senderPrimaryCommanderId: "$report.sender.commanders.primary.id",
              senderSecondaryCommanderId: "$report.sender.commanders.secondary.id",
              opponentPlayerId: "$report.opponent.player_id",
              opponentPlayerName: "$report.opponent.player_name",
              opponentKingdom: "$report.opponent.kingdom",
              opponentAlliance: "$report.opponent.alliance",
              opponentDuelId: "$report.opponent.duel_id",
              opponentAvatarUrl: "$report.opponent.avatar_url",
              opponentFrameUrl: "$report.opponent.frame_url",
              opponentPrimaryCommanderId: "$report.opponent.commanders.primary.id",
              opponentSecondaryCommanderId: "$report.opponent.commanders.secondary.id",
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
    .collection("duelbattle2Reports")
    .aggregate(aggregationPipeline, { allowDiskUse: true })
    .toArray();
  const hasMore = documents.length > 100;
  const finalDocuments = hasMore ? documents.slice(0, 100) : documents;
  const finalCursor: string | undefined = hasMore
    ? finalDocuments[finalDocuments.length - 1].latestMailTime.toString()
    : undefined;

  const items = finalDocuments.map((d) => ({
    duelId: d._id,
    count: d.count,
    winStreak: d.winStreak,
    emailTime: d.firstMailTime,
    entry: {
      sender: {
        playerId: d.firstDoc.senderPlayerId,
        playerName: d.firstDoc.senderPlayerName,
        kingdom: d.firstDoc.senderKingdom,
        alliance: d.firstDoc.senderAlliance,
        duelId: d.firstDoc.senderDuelId,
        avatarUrl: d.firstDoc.senderAvatarUrl,
        frameUrl: d.firstDoc.senderFrameUrl,
        primaryCommanderId: d.firstDoc.senderPrimaryCommanderId,
        secondaryCommanderId: d.firstDoc.senderSecondaryCommanderId,
      },
      opponent: {
        playerId: d.firstDoc.opponentPlayerId,
        playerName: d.firstDoc.opponentPlayerName,
        kingdom: d.firstDoc.opponentKingdom,
        alliance: d.firstDoc.opponentAlliance,
        duelId: d.firstDoc.opponentDuelId,
        avatarUrl: d.firstDoc.opponentAvatarUrl,
        frameUrl: d.firstDoc.opponentFrameUrl,
        primaryCommanderId: d.firstDoc.opponentPrimaryCommanderId,
        secondaryCommanderId: d.firstDoc.opponentSecondaryCommanderId,
      },
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
