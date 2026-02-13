import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import clientPromise from "@/lib/mongo";

function toFiniteNumber(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

export async function GET(req: NextRequest) {
  const searchParams = req.nextUrl.searchParams;

  const cursor = searchParams.get("cursor");

  const mongo = await clientPromise;
  const db = mongo.db();

  const matchPipeline: Document[] = [
    { "sender.duel.team_id": { $exists: true, $ne: null } },
    { "metadata.mail_time": { $exists: true, $ne: null } },
  ];

  const finalMatchPipeline =
    matchPipeline.length === 1 ? matchPipeline[0] : { $and: matchPipeline };

  const aggregationPipeline: Document[] = [
    { $match: finalMatchPipeline },
    {
      $group: {
        _id: "$sender.duel.team_id",
        count: { $sum: 1 },
        firstMailTime: { $min: "$metadata.mail_time" },
        latestMailTime: { $max: "$metadata.mail_time" },
        winStreak: {
          $sum: {
            $cond: [{ $eq: ["$battle_results.sender.win", true] }, 1, 0],
          },
        },
        senderKillCount: {
          $sum: {
            $add: [
              { $ifNull: ["$battle_results.sender.severely_wounded", 0] },
              { $ifNull: ["$battle_results.sender.dead", 0] },
            ],
          },
        },
        senderKillPoints: {
          $sum: {
            $ifNull: ["$battle_results.sender.kill_points", 0],
          },
        },
        opponentKillPoints: {
          $sum: {
            $ifNull: ["$battle_results.opponent.kill_points", 0],
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
        from: "mails_duelbattle2",
        let: { duelId: "$_id", fm: "$firstMailTime" },
        pipeline: [
          {
            $match: {
              $expr: {
                $and: [
                  { $eq: ["$sender.duel.team_id", "$$duelId"] },
                  { $eq: ["$metadata.mail_time", "$$fm"] },
                ],
              },
            },
          },
          { $sort: { "metadata.mail_time": 1 } },
          {
            $project: {
              senderPlayerId: "$sender.player_id",
              senderPlayerName: "$sender.player_name",
              senderAllianceAbbreviation: "$sender.alliance.abbreviation",
              senderDuelId: "$sender.duel.team_id",
              senderAvatarUrl: "$sender.avatar_url",
              senderFrameUrl: "$sender.frame_url",
              senderPrimaryCommanderId: "$sender.primary_commander.id",
              senderSecondaryCommanderId: "$sender.secondary_commander.id",
              opponentPlayerId: "$opponent.player_id",
              opponentPlayerName: "$opponent.player_name",
              opponentAllianceAbbreviation: "$opponent.alliance.abbreviation",
              opponentDuelId: "$opponent.duel.team_id",
              opponentAvatarUrl: "$opponent.avatar_url",
              opponentFrameUrl: "$opponent.frame_url",
              opponentPrimaryCommanderId: "$opponent.primary_commander.id",
              opponentSecondaryCommanderId: "$opponent.secondary_commander.id",
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
    .collection("mails_duelbattle2")
    .aggregate(aggregationPipeline, { allowDiskUse: true })
    .toArray();
  const hasMore = documents.length > 100;
  const finalDocuments = hasMore ? documents.slice(0, 100) : documents;
  const finalCursor: string | undefined = hasMore
    ? finalDocuments[finalDocuments.length - 1].latestMailTime.toString()
    : undefined;

  const items = finalDocuments.map((d) => ({
    duelId: toFiniteNumber(d._id),
    count: toFiniteNumber(d.count),
    winStreak: toFiniteNumber(d.winStreak),
    mailTime: toFiniteNumber(d.firstMailTime),
    killCount: toFiniteNumber(d.senderKillCount),
    tradePercent:
      toFiniteNumber(d.opponentKillPoints) > 0
        ? Math.round(
            (toFiniteNumber(d.senderKillPoints) / toFiniteNumber(d.opponentKillPoints)) * 100
          )
        : 0,
    entry: {
      sender: {
        playerId: toFiniteNumber(d.firstDoc.senderPlayerId),
        playerName: d.firstDoc.senderPlayerName ?? null,
        alliance: {
          abbreviation: d.firstDoc.senderAllianceAbbreviation ?? "",
        },
        duelId: toFiniteNumber(d.firstDoc.senderDuelId),
        avatarUrl: d.firstDoc.senderAvatarUrl ?? null,
        frameUrl: d.firstDoc.senderFrameUrl ?? null,
        primaryCommanderId: toFiniteNumber(d.firstDoc.senderPrimaryCommanderId),
        secondaryCommanderId: toFiniteNumber(d.firstDoc.senderSecondaryCommanderId),
      },
      opponent: {
        playerId: toFiniteNumber(d.firstDoc.opponentPlayerId),
        playerName: d.firstDoc.opponentPlayerName ?? null,
        alliance: {
          abbreviation: d.firstDoc.opponentAllianceAbbreviation ?? "",
        },
        duelId: toFiniteNumber(d.firstDoc.opponentDuelId),
        avatarUrl: d.firstDoc.opponentAvatarUrl ?? null,
        frameUrl: d.firstDoc.opponentFrameUrl ?? null,
        primaryCommanderId: toFiniteNumber(d.firstDoc.opponentPrimaryCommanderId),
        secondaryCommanderId: toFiniteNumber(d.firstDoc.opponentSecondaryCommanderId),
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
