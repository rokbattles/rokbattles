import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import { parseCursorTime } from "@/lib/parse-cursor-time";
import type {
  ExploreOlympianArenaPage,
  ExploreOlympianArenaPageDb,
} from "@/lib/types/explore-olympian-arena";
import type { MailsDuelBattle2Document } from "@/lib/types/mails-duelbattle2";

const EXPLORE_ARENA_PAGE_SIZE = 100;
const EXPLORE_ARENA_FETCH_SIZE = EXPLORE_ARENA_PAGE_SIZE + 1;

export const fetchExploreOlympianArena = cache(
  async function fetchExploreOlympianArena(
    afterParam?: string,
    beforeParam?: string
  ): Promise<ExploreOlympianArenaPage> {
    const beforeCursor = parseCursorTime(beforeParam);
    const afterCursor =
      beforeCursor !== null ? null : parseCursorTime(afterParam);
    let cursorMatch:
      | { first_mail_time: { $gt: number } }
      | { first_mail_time: { $lt: number } }
      | null = null;
    if (beforeCursor !== null) {
      cursorMatch = { first_mail_time: { $gt: beforeCursor } };
    } else if (afterCursor !== null) {
      cursorMatch = { first_mail_time: { $lt: afterCursor } };
    }

    const client = await clientPromise;

    if (!client) {
      return {
        rows: [],
        nextAfter: null,
        previousBefore: null,
      };
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
            opponent_kill_count_total: {
              $sum: {
                $add: [
                  {
                    $cond: [
                      { $isNumber: "$battle_results.opponent.dead" },
                      "$battle_results.opponent.dead",
                      0,
                    ],
                  },
                  {
                    $cond: [
                      {
                        $isNumber: "$battle_results.opponent.severely_wounded",
                      },
                      "$battle_results.opponent.severely_wounded",
                      0,
                    ],
                  },
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
        ...(cursorMatch ? [{ $match: cursorMatch }] : []),
        {
          $facet: {
            rows: [
              ...(beforeCursor !== null
                ? [
                    { $sort: { first_mail_time: 1, _id: 1 } },
                    { $limit: EXPLORE_ARENA_FETCH_SIZE },
                  ]
                : [
                    { $sort: { first_mail_time: -1, _id: -1 } },
                    { $limit: EXPLORE_ARENA_FETCH_SIZE },
                  ]),
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
                  kill_count: "$opponent_kill_count_total",
                  trade_percentage: 1,
                  win_streak: 1,
                },
              },
            ],
          },
        },
        {
          $project: {
            rows: 1,
          },
        },
      ])
      .next();

    if (!result) {
      return {
        rows: [],
        nextAfter: null,
        previousBefore: null,
      };
    }

    const hasMoreInQueryDirection =
      result.rows.length > EXPLORE_ARENA_PAGE_SIZE;
    const pagedRows = hasMoreInQueryDirection
      ? result.rows.slice(0, EXPLORE_ARENA_PAGE_SIZE)
      : result.rows;
    const orderedRows =
      beforeCursor !== null ? [...pagedRows].reverse() : pagedRows;

    const rows = orderedRows.map((row) => ({
      id: row._id.toString(),
      teamId: row._id,
      mailTime: row.first_mail_time,
      senderCommanders: row.sender_commanders,
      opponentCommanders: row.opponent_commanders,
      killCount: row.kill_count,
      tradePercentage: row.trade_percentage,
      winStreak: row.win_streak,
    }));

    const isInitialPage = afterCursor === null && beforeCursor === null;
    const firstRow = orderedRows.at(0);
    const lastRow = orderedRows.at(-1);

    const previousBefore =
      firstRow &&
      !isInitialPage &&
      (afterCursor !== null ||
        (beforeCursor !== null && hasMoreInQueryDirection))
        ? firstRow.first_mail_time.toString()
        : null;
    const nextAfter =
      lastRow && (beforeCursor !== null || hasMoreInQueryDirection)
        ? lastRow.first_mail_time.toString()
        : null;

    return { rows, nextAfter, previousBefore };
  }
);
