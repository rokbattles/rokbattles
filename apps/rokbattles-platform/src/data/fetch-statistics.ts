import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import type { MailsBarCanyonKillBossDocument } from "@/lib/types/mails-barcanyonkillboss";
import type { MailsBattleDocument } from "@/lib/types/mails-battle";
import type { MailsDuelBattle2Document } from "@/lib/types/mails-duelbattle2";
import type {
  Statistics,
  StatisticsBarCanyonAggregationDb,
  StatisticsBattleAggregationDb,
} from "@/lib/types/statistics";

const EMPTY_STATISTICS: Statistics = {
  battle: 0,
  battle_npc: 0,
  duelbattle2: 0,
  barcanyonkillboss: 0,
};

export const fetchStatistics = cache(
  async function fetchStatistics(): Promise<Statistics> {
    try {
      const client = await clientPromise;

      if (!client) {
        return EMPTY_STATISTICS;
      }

      const db = client.db();

      const [battleStats, duelbattle2Count, barCanyonStats] = await Promise.all(
        [
          db
            .collection<MailsBattleDocument>("mails_battle")
            .aggregate<StatisticsBattleAggregationDb>([
              {
                $project: {
                  battle: {
                    $size: {
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
                  },
                  battle_npc: {
                    $size: {
                      $filter: {
                        input: { $ifNull: ["$opponents", []] },
                        as: "opponent",
                        cond: { $eq: ["$$opponent.player_id", -2] },
                      },
                    },
                  },
                },
              },
              {
                $group: {
                  _id: null,
                  battle: { $sum: "$battle" },
                  battle_npc: { $sum: "$battle_npc" },
                },
              },
              {
                $project: {
                  _id: 0,
                  battle: 1,
                  battle_npc: 1,
                },
              },
            ])
            .next(),
          db
            .collection<MailsDuelBattle2Document>("mails_duelbattle2")
            .countDocuments(),
          db
            .collection<MailsBarCanyonKillBossDocument>(
              "mails_barcanyonkillboss"
            )
            .aggregate<StatisticsBarCanyonAggregationDb>([
              {
                $project: {
                  participant_count: {
                    $size: { $ifNull: ["$participants", []] },
                  },
                },
              },
              {
                $group: {
                  _id: null,
                  barcanyonkillboss: { $sum: "$participant_count" },
                },
              },
              {
                $project: {
                  _id: 0,
                  barcanyonkillboss: 1,
                },
              },
            ])
            .next(),
        ]
      );

      return {
        battle: battleStats?.battle ?? 0,
        battle_npc: battleStats?.battle_npc ?? 0,
        duelbattle2: duelbattle2Count ?? 0,
        barcanyonkillboss: barCanyonStats?.barcanyonkillboss ?? 0,
      };
    } catch {
      return EMPTY_STATISTICS;
    }
  }
);
