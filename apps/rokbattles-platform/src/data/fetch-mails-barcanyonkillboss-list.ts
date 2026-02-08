import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import type {
  MailsBarCanyonKillBossDocument,
  MailsBarCanyonKillBossList,
  MailsBarCanyonKillBossListDb,
} from "@/lib/types/mails-barcanyonkillboss";

export const fetchMailsBarCanyonKillBossList = cache(
  async function fetchMailsBarCanyonKillBossList(
    pageParam: number,
    sizeParam: number
  ): Promise<MailsBarCanyonKillBossList> {
    const page = Math.max(1, Math.trunc(pageParam));
    const size = Math.min(100, Math.max(1, Math.trunc(sizeParam)));
    const skip = (page - 1) * size;

    const client = await clientPromise;

    if (!client) {
      return { rows: [], total: 0 };
    }

    const db = client.db();

    const result = await db
      .collection<MailsBarCanyonKillBossDocument>("mails_barcanyonkillboss")
      .aggregate<MailsBarCanyonKillBossListDb>([
        {
          $facet: {
            rows: [
              { $sort: { "metadata.mail_time": -1, _id: -1 } },
              { $skip: skip },
              { $limit: size },
              {
                $project: {
                  _id: 1,
                  metadata: 1,
                  npc: 1,
                  participants: 1,
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
        metadata: row.metadata,
        npc: row.npc,
        participants: row.participants,
      })),
      total: result.total,
    };
  }
);
