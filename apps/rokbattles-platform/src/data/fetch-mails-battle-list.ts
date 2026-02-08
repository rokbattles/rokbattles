import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import type {
  MailsBattleDocument,
  MailsBattleList,
  MailsBattleListDb,
} from "@/lib/types/mails-battle";

export const fetchMailsBattleList = cache(async function fetchMailsBattleList(
  pageParam: number,
  sizeParam: number
): Promise<MailsBattleList> {
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
    .aggregate<MailsBattleListDb>([
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
                sender: {
                  player_id: "$sender.player_id",
                  player_name: "$sender.player_name",
                  kingdom_id: "$sender.kingdom_id",
                  alliance: "$sender.alliance",
                  alliance_building_id: "$sender.alliance_building_id",
                  tracking_key: "$sender.tracking_key",
                  app_id: "$sender.app_id",
                  app_uid: "$sender.app_uid",
                  avatar_url: "$sender.avatar_url",
                  frame_url: "$sender.frame_url",
                  participants: "$sender.participants",
                },
                opponents: {
                  $map: {
                    input: { $ifNull: ["$opponents", []] },
                    as: "opponent",
                    in: {
                      player_id: "$$opponent.player_id",
                      player_name: "$$opponent.player_name",
                      kingdom_id: "$$opponent.kingdom_id",
                      alliance: "$$opponent.alliance",
                      alliance_building_id: "$$opponent.alliance_building_id",
                      tracking_key: "$$opponent.tracking_key",
                      app_id: "$$opponent.app_id",
                      app_uid: "$$opponent.app_uid",
                      avatar_url: "$$opponent.avatar_url",
                      frame_url: "$$opponent.frame_url",
                      participants: "$$opponent.participants",
                      attack: "$$opponent.attack",
                    },
                  },
                },
                summary: 1,
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
      sender: row.sender,
      opponents: row.opponents,
      summary: row.summary,
    })),
    total: result.total,
  };
});
