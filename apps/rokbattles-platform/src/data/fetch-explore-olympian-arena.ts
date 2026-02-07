import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import type {
  ExploreOlympianArenaPage,
  ExploreOlympianArenaPageDb,
} from "@/lib/types/explore-olympian-arena";
import type { MailsDuelBattle2Document } from "@/lib/types/mails-duelbattle2";

export const fetchExploreOlympianArena = cache(
  async function fetchExploreOlympianArena(
    pageParam: number,
    sizeParam: number
  ): Promise<ExploreOlympianArenaPage> {
    const page = Math.max(1, Math.trunc(pageParam));
    const size = Math.min(100, Math.max(1, Math.trunc(sizeParam)));
    const skip = (page - 1) * size;

    const client = await clientPromise;

    if (!client) {
      return { rows: [], total: 0 };
    }

    const db = client.db();

    const result = await db
      .collection<MailsDuelBattle2Document>("mails_duelbattle2")
      .aggregate<ExploreOlympianArenaPageDb>([
        {
          $match: {
            "sender.duel.team_id": { $exists: true, $ne: null },
          },
        },
        { $sort: { "metadata.mail_time": 1, _id: 1 } },
        {
          $group: {
            _id: "$sender.duel.team_id",
            first_mail_time: { $first: "$metadata.mail_time" },
            latest_mail_time: { $last: "$metadata.mail_time" },
            sender_primary: { $first: "$sender.primary_commander.id" },
            sender_secondary: { $first: "$sender.secondary_commander.id" },
            opponent_primary: { $first: "$opponent.primary_commander.id" },
            opponent_secondary: { $first: "$opponent.secondary_commander.id" },
            battles: { $sum: 1 },
          },
        },
        {
          $addFields: {
            trade_percentage: 0,
            win_streak: {
              $max: [{ $subtract: ["$battles", 1] }, 0],
            },
          },
        },
        {
          $facet: {
            rows: [
              { $sort: { latest_mail_time: -1, _id: -1 } },
              { $skip: skip },
              { $limit: size },
              {
                $project: {
                  _id: 1,
                  first_mail_time: 1,
                  sender_commanders: {
                    primary: { $ifNull: ["$sender_primary", null] },
                    secondary: { $ifNull: ["$sender_secondary", null] },
                  },
                  opponent_commanders: {
                    primary: { $ifNull: ["$opponent_primary", null] },
                    secondary: { $ifNull: ["$opponent_secondary", null] },
                  },
                  trade_percentage: 1,
                  win_streak: 1,
                },
              },
            ],
            total: [{ $count: "value" }],
          },
        },
        {
          $project: {
            rows: 1,
            total: { $ifNull: [{ $first: "$total.value" }, 0] },
          },
        },
      ])
      .next();

    if (!result) {
      return { rows: [], total: 0 };
    }

    return {
      rows: result.rows.map((row) => ({
        id: row._id.toString(),
        teamId: row._id,
        mailTime: row.first_mail_time ?? null,
        senderCommanders: row.sender_commanders,
        opponentCommanders: row.opponent_commanders,
        tradePercentage: row.trade_percentage,
        winStreak: row.win_streak,
      })),
      total: result.total,
    };
  }
);
