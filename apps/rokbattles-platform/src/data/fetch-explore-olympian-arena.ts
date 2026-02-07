import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import { parseObjectId } from "@/lib/parse-object-id";
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
    const beforeCursor = parseObjectId(beforeParam);
    const afterCursor = beforeCursor ? null : parseObjectId(afterParam);
    let cursorMatch:
      | { latest_doc_id: { $gt: typeof beforeCursor } }
      | { latest_doc_id: { $lt: typeof afterCursor } }
      | null = null;
    if (beforeCursor) {
      cursorMatch = { latest_doc_id: { $gt: beforeCursor } };
    } else if (afterCursor) {
      cursorMatch = { latest_doc_id: { $lt: afterCursor } };
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
            latest_mail_time: { $last: "$metadata.mail_time" },
            latest_doc_id: { $last: "$_id" },
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
        ...(cursorMatch ? [{ $match: cursorMatch }] : []),
        {
          $facet: {
            rows: [
              ...(beforeCursor
                ? [
                    { $sort: { latest_doc_id: 1, _id: 1 } },
                    { $limit: EXPLORE_ARENA_FETCH_SIZE },
                  ]
                : [
                    { $sort: { latest_doc_id: -1, _id: -1 } },
                    { $limit: EXPLORE_ARENA_FETCH_SIZE },
                  ]),
              {
                $project: {
                  _id: 1,
                  first_mail_time: 1,
                  latest_doc_id: 1,
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
    const orderedRows = beforeCursor ? [...pagedRows].reverse() : pagedRows;

    const rows = orderedRows.map((row) => ({
      id: row._id.toString(),
      teamId: row._id,
      mailTime: row.first_mail_time,
      senderCommanders: row.sender_commanders,
      opponentCommanders: row.opponent_commanders,
      tradePercentage: row.trade_percentage,
      winStreak: row.win_streak,
    }));

    const isInitialPage = !(afterCursor || beforeCursor);
    const firstRow = orderedRows.at(0);
    const lastRow = orderedRows.at(-1);

    const previousBefore =
      firstRow &&
      !isInitialPage &&
      (afterCursor || (beforeCursor && hasMoreInQueryDirection))
        ? firstRow.latest_doc_id.toString()
        : null;
    const nextAfter =
      lastRow && (beforeCursor || hasMoreInQueryDirection)
        ? lastRow.latest_doc_id.toString()
        : null;

    return { rows, nextAfter, previousBefore };
  }
);
