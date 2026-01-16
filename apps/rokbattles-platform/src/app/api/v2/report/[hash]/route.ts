import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import clientPromise, { toPlainObject } from "@/lib/mongo";
import type { RawReportPayload } from "@/lib/types/raw-report";
import type {
  BattleResultsSummary,
  BattleResultsTotals,
  ReportEntry,
} from "@/lib/types/report";

export async function GET(
  _req: NextRequest,
  ctx: RouteContext<"/api/v2/report/[hash]">
) {
  const { hash } = await ctx.params;
  if (!hash) {
    return NextResponse.json({ error: "Missing report hash" }, { status: 400 });
  }

  try {
    const mongo = await clientPromise;
    const db = mongo.db();

    const matchPipeline: Document = {
      "metadata.parentHash": hash,
      "report.enemy.player_id": { $nin: [-2, 0] },
    };

    const aggregationPipeline: Document[] = [
      { $match: matchPipeline },
      {
        $project: {
          _id: 0,
          startDate: "$report.metadata.start_date",
          report: "$report",
        },
      },
      { $sort: { startDate: 1 } },
    ];

    const documents = await db
      .collection("battleReports")
      .aggregate(aggregationPipeline, { allowDiskUse: true })
      .toArray();

    let trackingKey: string | null = null;
    const items: ReportEntry[] = documents.map((doc) => {
      const report = toPlainObject(doc.report);
      if (!trackingKey) {
        trackingKey = extractTrackingKey(report);
      }
      return {
        startDate: Number(doc.startDate),
        report,
      };
    });

    let merge:
      | {
          trackingKey: string;
          reports: { parentHash: string; latestEmailTime: number }[];
        }
      | undefined;

    if (trackingKey) {
      const mergePipeline: Document[] = [
        {
          $match: {
            "report.self.tracking_key": trackingKey,
            "report.enemy.player_id": { $nin: [-2, 0] },
            "metadata.parentHash": { $exists: true, $ne: null },
          },
        },
        {
          $group: {
            _id: "$metadata.parentHash",
            latestEmailTime: { $max: "$report.metadata.email_time" },
          },
        },
        { $sort: { latestEmailTime: -1 } },
      ];

      const mergeDocs = await db
        .collection("battleReports")
        .aggregate(mergePipeline, { allowDiskUse: true })
        .toArray();

      const mergeReports = mergeDocs
        .map((doc) => ({
          parentHash: String(doc._id ?? ""),
          latestEmailTime: Number(doc.latestEmailTime ?? 0),
        }))
        .filter((doc) => doc.parentHash.length > 0);

      if (mergeReports.length > 1) {
        merge = {
          trackingKey,
          reports: mergeReports,
        };
      }
    }

    const totalsPipeline: Document[] = [
      { $match: matchPipeline },
      {
        $group: {
          _id: null,
          death: { $sum: { $ifNull: ["$report.battle_results.death", 0] } },
          severelyWounded: {
            $sum: { $ifNull: ["$report.battle_results.severely_wounded", 0] },
          },
          wounded: { $sum: { $ifNull: ["$report.battle_results.wounded", 0] } },
          remaining: { $min: "$report.battle_results.remaining" },
          killScore: {
            $sum: { $ifNull: ["$report.battle_results.kill_score", 0] },
          },
          enemyDeath: {
            $sum: { $ifNull: ["$report.battle_results.enemy_death", 0] },
          },
          enemySeverelyWounded: {
            $sum: {
              $ifNull: ["$report.battle_results.enemy_severely_wounded", 0],
            },
          },
          enemyWounded: {
            $sum: { $ifNull: ["$report.battle_results.enemy_wounded", 0] },
          },
          enemyRemaining: {
            $sum: { $ifNull: ["$report.battle_results.enemy_remaining", 0] },
          },
          enemyKillScore: {
            $sum: { $ifNull: ["$report.battle_results.enemy_kill_score", 0] },
          },
        },
      },
      {
        $project: {
          _id: 0,
          death: 1,
          severelyWounded: 1,
          wounded: 1,
          remaining: 1,
          killScore: 1,
          enemyDeath: 1,
          enemySeverelyWounded: 1,
          enemyWounded: 1,
          enemyRemaining: 1,
          enemyKillScore: 1,
        },
      },
    ];

    let totals: BattleResultsTotals | undefined;
    try {
      const totalsDocs = await db
        .collection("battleReports")
        .aggregate(totalsPipeline, { allowDiskUse: true })
        .toArray();
      const totalsDoc = totalsDocs[0];
      if (totalsDoc) {
        totals = {
          death: Number(totalsDoc.death),
          severelyWounded: Number(totalsDoc.severelyWounded),
          wounded: Number(totalsDoc.wounded),
          remaining: Number(totalsDoc.remaining),
          killScore: Number(totalsDoc.killScore),
          enemyDeath: Number(totalsDoc.enemyDeath),
          enemySeverelyWounded: Number(totalsDoc.enemySeverelyWounded),
          enemyWounded: Number(totalsDoc.enemyWounded),
          enemyRemaining: Number(totalsDoc.enemyRemaining),
          enemyKillScore: Number(totalsDoc.enemyKillScore),
        };
      }
    } catch (error) {
      console.error("Failed to compute report totals", error);
    }

    const battleResults: BattleResultsSummary | undefined = totals
      ? { total: totals }
      : undefined;

    return NextResponse.json(
      {
        parentHash: hash,
        items,
        count: items.length,
        ...(battleResults ? { battleResults } : {}),
        ...(merge ? { merge } : {}),
      },
      {
        headers: {
          "Cache-Control": "public, max-age=604800, immutable",
        },
      }
    );
  } catch (error) {
    console.error("Failed to load reports by hash", error);
    return NextResponse.json(
      { error: "Failed to load reports" },
      {
        status: 500,
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  }
}

function extractTrackingKey(
  report: Record<string, unknown> | null | undefined
): string | null {
  const payload = report as RawReportPayload | null | undefined;
  const candidate = payload?.self?.tracking_key;
  if (typeof candidate !== "string") {
    return null;
  }
  const trimmed = candidate.trim();
  return trimmed.length > 0 ? trimmed : null;
}
