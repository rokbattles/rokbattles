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
            fallback_sender_summary: {
              $reduce: {
                input: { $ifNull: ["$opponents", []] },
                initialValue: {
                  kill_points: 0,
                  dead: 0,
                  severely_wounded: 0,
                  slightly_wounded: 0,
                  remaining: 0,
                  troop_units: 0,
                },
                in: {
                  kill_points: {
                    $add: [
                      "$$value.kill_points",
                      {
                        $cond: [
                          {
                            $isNumber:
                              "$$this.battle_results.sender.kill_points",
                          },
                          "$$this.battle_results.sender.kill_points",
                          0,
                        ],
                      },
                    ],
                  },
                  dead: {
                    $add: [
                      "$$value.dead",
                      {
                        $cond: [
                          { $isNumber: "$$this.battle_results.sender.dead" },
                          "$$this.battle_results.sender.dead",
                          0,
                        ],
                      },
                    ],
                  },
                  severely_wounded: {
                    $add: [
                      "$$value.severely_wounded",
                      {
                        $cond: [
                          {
                            $isNumber:
                              "$$this.battle_results.sender.severely_wounded",
                          },
                          "$$this.battle_results.sender.severely_wounded",
                          0,
                        ],
                      },
                    ],
                  },
                  slightly_wounded: {
                    $add: [
                      "$$value.slightly_wounded",
                      {
                        $cond: [
                          {
                            $isNumber:
                              "$$this.battle_results.sender.slightly_wounded",
                          },
                          "$$this.battle_results.sender.slightly_wounded",
                          0,
                        ],
                      },
                    ],
                  },
                  remaining: {
                    $add: [
                      "$$value.remaining",
                      {
                        $cond: [
                          {
                            $isNumber: "$$this.battle_results.sender.remaining",
                          },
                          "$$this.battle_results.sender.remaining",
                          0,
                        ],
                      },
                    ],
                  },
                  troop_units: {
                    $add: [
                      "$$value.troop_units",
                      {
                        $cond: [
                          {
                            $isNumber:
                              "$$this.battle_results.sender.troop_units",
                          },
                          "$$this.battle_results.sender.troop_units",
                          0,
                        ],
                      },
                    ],
                  },
                },
              },
            },
            fallback_opponent_summary: {
              $reduce: {
                input: { $ifNull: ["$opponents", []] },
                initialValue: {
                  kill_points: 0,
                  dead: 0,
                  severely_wounded: 0,
                  slightly_wounded: 0,
                  remaining: 0,
                  troop_units: 0,
                },
                in: {
                  kill_points: {
                    $add: [
                      "$$value.kill_points",
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
                  dead: {
                    $add: [
                      "$$value.dead",
                      {
                        $cond: [
                          { $isNumber: "$$this.battle_results.opponent.dead" },
                          "$$this.battle_results.opponent.dead",
                          0,
                        ],
                      },
                    ],
                  },
                  severely_wounded: {
                    $add: [
                      "$$value.severely_wounded",
                      {
                        $cond: [
                          {
                            $isNumber:
                              "$$this.battle_results.opponent.severely_wounded",
                          },
                          "$$this.battle_results.opponent.severely_wounded",
                          0,
                        ],
                      },
                    ],
                  },
                  slightly_wounded: {
                    $add: [
                      "$$value.slightly_wounded",
                      {
                        $cond: [
                          {
                            $isNumber:
                              "$$this.battle_results.opponent.slightly_wounded",
                          },
                          "$$this.battle_results.opponent.slightly_wounded",
                          0,
                        ],
                      },
                    ],
                  },
                  remaining: {
                    $add: [
                      "$$value.remaining",
                      {
                        $cond: [
                          {
                            $isNumber:
                              "$$this.battle_results.opponent.remaining",
                          },
                          "$$this.battle_results.opponent.remaining",
                          0,
                        ],
                      },
                    ],
                  },
                  troop_units: {
                    $add: [
                      "$$value.troop_units",
                      {
                        $cond: [
                          {
                            $isNumber:
                              "$$this.battle_results.opponent.troop_units",
                          },
                          "$$this.battle_results.opponent.troop_units",
                          0,
                        ],
                      },
                    ],
                  },
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
            sender_summary: {
              kill_points: {
                $cond: [
                  { $isNumber: "$summary.sender.kill_points" },
                  "$summary.sender.kill_points",
                  "$fallback_sender_summary.kill_points",
                ],
              },
              dead: {
                $cond: [
                  { $isNumber: "$summary.sender.dead" },
                  "$summary.sender.dead",
                  "$fallback_sender_summary.dead",
                ],
              },
              severely_wounded: {
                $cond: [
                  { $isNumber: "$summary.sender.severely_wounded" },
                  "$summary.sender.severely_wounded",
                  "$fallback_sender_summary.severely_wounded",
                ],
              },
              slightly_wounded: {
                $cond: [
                  { $isNumber: "$summary.sender.slightly_wounded" },
                  "$summary.sender.slightly_wounded",
                  "$fallback_sender_summary.slightly_wounded",
                ],
              },
              remaining: {
                $cond: [
                  { $isNumber: "$summary.sender.remaining" },
                  "$summary.sender.remaining",
                  "$fallback_sender_summary.remaining",
                ],
              },
              troop_units: {
                $cond: [
                  { $isNumber: "$summary.sender.troop_units" },
                  "$summary.sender.troop_units",
                  "$fallback_sender_summary.troop_units",
                ],
              },
            },
            opponent_summary: {
              kill_points: {
                $cond: [
                  { $isNumber: "$summary.opponent.kill_points" },
                  "$summary.opponent.kill_points",
                  "$fallback_opponent_summary.kill_points",
                ],
              },
              dead: {
                $cond: [
                  { $isNumber: "$summary.opponent.dead" },
                  "$summary.opponent.dead",
                  "$fallback_opponent_summary.dead",
                ],
              },
              severely_wounded: {
                $cond: [
                  { $isNumber: "$summary.opponent.severely_wounded" },
                  "$summary.opponent.severely_wounded",
                  "$fallback_opponent_summary.severely_wounded",
                ],
              },
              slightly_wounded: {
                $cond: [
                  { $isNumber: "$summary.opponent.slightly_wounded" },
                  "$summary.opponent.slightly_wounded",
                  "$fallback_opponent_summary.slightly_wounded",
                ],
              },
              remaining: {
                $cond: [
                  { $isNumber: "$summary.opponent.remaining" },
                  "$summary.opponent.remaining",
                  "$fallback_opponent_summary.remaining",
                ],
              },
              troop_units: {
                $cond: [
                  { $isNumber: "$summary.opponent.troop_units" },
                  "$summary.opponent.troop_units",
                  "$fallback_opponent_summary.troop_units",
                ],
              },
            },
          },
        },
        {
          $addFields: {
            sender_kp: "$sender_summary.kill_points",
            opponent_kp: "$opponent_summary.kill_points",
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
                  summary: {
                    sender: "$sender_summary",
                    opponent: "$opponent_summary",
                  },
                  timeline: {
                    start_timestamp: "$timeline.start_timestamp",
                    end_timestamp: "$timeline.end_timestamp",
                    sampling: {
                      $map: {
                        input: {
                          $filter: {
                            input: { $ifNull: ["$timeline.sampling", []] },
                            as: "sample",
                            cond: {
                              $and: [
                                { $isNumber: "$$sample.tick" },
                                { $isNumber: "$$sample.count" },
                              ],
                            },
                          },
                        },
                        as: "sample",
                        in: {
                          tick: "$$sample.tick",
                          count: "$$sample.count",
                        },
                      },
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
        summary: {
          sender: {
            dead: row.summary.sender.dead,
            killPoints: row.summary.sender.kill_points,
            remaining: row.summary.sender.remaining,
            severelyWounded: row.summary.sender.severely_wounded,
            slightlyWounded: row.summary.sender.slightly_wounded,
            troopUnits: row.summary.sender.troop_units,
          },
          opponent: {
            dead: row.summary.opponent.dead,
            killPoints: row.summary.opponent.kill_points,
            remaining: row.summary.opponent.remaining,
            severelyWounded: row.summary.opponent.severely_wounded,
            slightlyWounded: row.summary.opponent.slightly_wounded,
            troopUnits: row.summary.opponent.troop_units,
          },
        },
        timeline: {
          startTimestamp: row.timeline.start_timestamp,
          endTimestamp: row.timeline.end_timestamp,
          sampling: row.timeline.sampling,
        },
      })),
      total: result.total,
    };
  }
);
