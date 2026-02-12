import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import type {
  MailsDuelBattle2Document,
  MailsDuelBattle2List,
  MailsDuelBattle2ListDb,
} from "@/lib/types/mails-duelbattle2";

export const fetchMailsDuelBattle2List = cache(
  async function fetchMailsDuelBattle2List(
    pageParam: number,
    sizeParam: number
  ): Promise<MailsDuelBattle2List> {
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
      .aggregate<MailsDuelBattle2ListDb>([
        {
          $match: {
            "sender.duel.team_id": { $exists: true, $ne: null },
          },
        },
        {
          $facet: {
            rows: [
              { $sort: { "metadata.mail_time": -1, _id: -1 } },
              {
                $group: {
                  _id: "$sender.duel.team_id",
                  latest_mail_time: { $first: "$metadata.mail_time" },
                  count: { $sum: 1 },
                  reports: {
                    $push: {
                      _id: "$_id",
                      metadata: "$metadata",
                      sender: "$sender",
                      opponent: "$opponent",
                    },
                  },
                },
              },
              { $sort: { latest_mail_time: -1, _id: -1 } },
              { $skip: skip },
              { $limit: size },
              {
                $project: {
                  _id: 0,
                  team_id: "$_id",
                  latest_mail_time: 1,
                  count: 1,
                  reports: 1,
                },
              },
            ],
            total: [
              { $group: { _id: "$sender.duel.team_id" } },
              { $count: "value" },
            ],
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
        team_id: row.team_id,
        latest_mail_time: row.latest_mail_time,
        count: row.count,
        reports: row.reports.map((report) => ({
          id: report._id.toString(),
          metadata: report.metadata,
          sender: report.sender,
          opponent: report.opponent,
        })),
      })),
      total: result.total,
    };
  }
);
