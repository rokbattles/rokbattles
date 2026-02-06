import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import type {
  ExploreBattleReportsPage,
  ExploreBattleReportsPageDb,
} from "@/lib/types/explore-battle-reports";
import type { MailsBattleDocument } from "@/lib/types/mails-battle";

export const fetchExploreBattleReports = cache(
  async function fetchExploreBattleReports(
    pageParam: number,
    sizeParam: number
  ): Promise<ExploreBattleReportsPage> {
    const page = Math.max(1, Math.trunc(pageParam));
    const size = Math.min(100, Math.max(1, Math.trunc(sizeParam)));
    const skip = (page - 1) * size;

    const client = await clientPromise;

    if (!client) {
      return { rows: [], total: 0 };
    }

    const db = client.db();

    const result = await db
      .collection<MailsBattleDocument>("mails_battle")
      .aggregate<ExploreBattleReportsPageDb>([
        {
          $addFields: {
            valid_opponents: {
              $filter: {
                input: { $ifNull: ["$opponents", []] },
                as: "opponent",
                cond: {
                  $and: [
                    { $ne: ["$$opponent.player_id", 0] },
                    { $ne: ["$$opponent.player_id", -2] },
                    { $ne: ["$$opponent.player_id", null] },
                  ],
                },
              },
            },
            fallback_sender_kp: {
              $reduce: {
                input: { $ifNull: ["$opponents", []] },
                initialValue: 0,
                in: {
                  $add: [
                    "$$value",
                    {
                      $cond: [
                        {
                          $isNumber: "$$this.battle_results.sender.kill_points",
                        },
                        "$$this.battle_results.sender.kill_points",
                        0,
                      ],
                    },
                  ],
                },
              },
            },
            fallback_opponent_kp: {
              $reduce: {
                input: { $ifNull: ["$opponents", []] },
                initialValue: 0,
                in: {
                  $add: [
                    "$$value",
                    {
                      $cond: [
                        {
                          $isNumber:
                            "$$this.battle_results.opponent.kill_points",
                        },
                        "$$this.battle_results.opponent.kill_points",
                        0,
                      ],
                    },
                  ],
                },
              },
            },
          },
        },
        {
          $addFields: {
            battles: {
              $size: "$valid_opponents",
            },
            preferred_opponent: {
              $let: {
                vars: {
                  garrison_opponents: {
                    $filter: {
                      input: "$valid_opponents",
                      as: "opponent",
                      cond: {
                        $or: [
                          { $ne: ["$$opponent.alliance_building_id", null] },
                          { $ne: ["$$opponent.structure_id", null] },
                        ],
                      },
                    },
                  },
                },
                in: {
                  $ifNull: [
                    { $arrayElemAt: ["$$garrison_opponents", 0] },
                    { $arrayElemAt: ["$valid_opponents", 0] },
                  ],
                },
              },
            },
            sender_kp: {
              $cond: [
                {
                  $and: [
                    { $isNumber: "$summary.sender.kill_points" },
                    { $isNumber: "$summary.opponent.kill_points" },
                  ],
                },
                "$summary.sender.kill_points",
                "$fallback_sender_kp",
              ],
            },
            opponent_kp: {
              $cond: [
                {
                  $and: [
                    { $isNumber: "$summary.sender.kill_points" },
                    { $isNumber: "$summary.opponent.kill_points" },
                  ],
                },
                "$summary.opponent.kill_points",
                "$fallback_opponent_kp",
              ],
            },
          },
        },
        {
          $match: {
            battles: { $gt: 0 },
          },
        },
        {
          $addFields: {
            trade_percentage: {
              $round: [
                {
                  $cond: [
                    { $gt: ["$opponent_kp", 0] },
                    {
                      $multiply: [
                        { $divide: ["$sender_kp", "$opponent_kp"] },
                        100,
                      ],
                    },
                    {
                      $cond: [{ $eq: ["$sender_kp", "$opponent_kp"] }, 100, 0],
                    },
                  ],
                },
                0,
              ],
            },
          },
        },
        {
          $facet: {
            rows: [
              { $sort: { "metadata.mail_time": -1, _id: -1 } },
              { $skip: skip },
              { $limit: size },
              {
                $project: {
                  _id: 1,
                  mail_id: "$metadata.mail_id",
                  start_timestamp: "$timeline.start_timestamp",
                  end_timestamp: "$timeline.end_timestamp",
                  sender_commanders: {
                    primary: "$sender.commanders.primary.id",
                    secondary: "$sender.commanders.secondary.id",
                  },
                  opponent_commanders: {
                    primary: {
                      $ifNull: [
                        "$preferred_opponent.commanders.primary.id",
                        null,
                      ],
                    },
                    secondary: {
                      $ifNull: [
                        "$preferred_opponent.commanders.secondary.id",
                        null,
                      ],
                    },
                  },
                  trade_percentage: 1,
                  battles: 1,
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
        mailId: row.mail_id ?? row._id.toString(),
        startTimestamp: row.start_timestamp ?? null,
        endTimestamp: row.end_timestamp ?? null,
        senderCommanders: row.sender_commanders,
        opponentCommanders: row.opponent_commanders,
        tradePercentage: row.trade_percentage,
        battles: row.battles,
      })),
      total: result.total,
    };
  }
);
