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
            "metadata.mail_time": { $exists: true, $ne: null },
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
            sender_kp_total: {
              $sum: {
                $cond: [
                  { $isNumber: "$battle_results.sender.kill_points" },
                  "$battle_results.sender.kill_points",
                  0,
                ],
              },
            },
            opponent_kp_total: {
              $sum: {
                $cond: [
                  { $isNumber: "$battle_results.opponent.kill_points" },
                  "$battle_results.opponent.kill_points",
                  0,
                ],
              },
            },
            sender_wins: {
              $push: { $eq: ["$battle_results.sender.win", true] },
            },
          },
        },
        {
          $addFields: {
            trade_percentage: {
              $round: [
                {
                  $cond: [
                    { $gt: ["$opponent_kp_total", 0] },
                    {
                      $multiply: [
                        { $divide: ["$sender_kp_total", "$opponent_kp_total"] },
                        100,
                      ],
                    },
                    {
                      $cond: [
                        { $eq: ["$sender_kp_total", "$opponent_kp_total"] },
                        100,
                        0,
                      ],
                    },
                  ],
                },
                0,
              ],
            },
            win_streak: {
              $let: {
                vars: {
                  first_loss_index: { $indexOfArray: ["$sender_wins", false] },
                },
                in: {
                  $cond: [
                    { $eq: ["$$first_loss_index", -1] },
                    { $size: "$sender_wins" },
                    "$$first_loss_index",
                  ],
                },
              },
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
                    primary: "$sender_primary",
                    secondary: "$sender_secondary",
                  },
                  opponent_commanders: {
                    primary: "$opponent_primary",
                    secondary: "$opponent_secondary",
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
        mailTime: row.first_mail_time,
        senderCommanders: row.sender_commanders,
        opponentCommanders: row.opponent_commanders,
        tradePercentage: row.trade_percentage,
        winStreak: row.win_streak,
      })),
      total: result.total,
    };
  }
);
