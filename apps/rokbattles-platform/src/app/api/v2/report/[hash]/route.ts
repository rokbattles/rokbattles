import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import client from "@/lib/mongo";
import type { BattleResultsSummary, BattleResultsTotals, ReportEntry } from "@/lib/types/report";

function toPlainObject(source: unknown): Record<string, unknown> {
  if (!source || typeof source !== "object") {
    return {};
  }

  const serialized = JSON.stringify(source, (_, value) => {
    if (
      value &&
      typeof value === "object" &&
      "$numberLong" in (value as Record<string, unknown>) &&
      typeof (value as Record<string, unknown>).$numberLong === "string"
    ) {
      const numericValue = Number((value as { $numberLong: string }).$numberLong);
      return Number.isFinite(numericValue)
        ? numericValue
        : (value as { $numberLong: string }).$numberLong;
    }
    return value;
  });

  return JSON.parse(serialized) as Record<string, unknown>;
}

export async function GET(_req: NextRequest, ctx: RouteContext<"/api/v2/report/[hash]">) {
  const { hash } = await ctx.params;
  if (!hash) {
    return NextResponse.json({ error: "Missing report hash" }, { status: 400 });
  }

  try {
    const mongo = await client.connect();
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
          hash: "$metadata.hash",
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

    const items: ReportEntry[] = documents.map((doc) => ({
      hash: typeof doc.hash === "string" ? doc.hash : String(doc.hash ?? ""),
      startDate: Number(doc.startDate),
      report: toPlainObject(doc.report),
    }));

    const totalsPipeline: Document[] = [
      { $match: matchPipeline },
      {
        $group: {
          _id: null,
          death: { $sum: { $ifNull: ["$report.battle_results.death", 0] } },
          severelyWounded: { $sum: { $ifNull: ["$report.battle_results.severely_wounded", 0] } },
          wounded: { $sum: { $ifNull: ["$report.battle_results.wounded", 0] } },
          remaining: { $min: "$report.battle_results.remaining" },
          killScore: { $sum: { $ifNull: ["$report.battle_results.kill_score", 0] } },
          enemyDeath: { $sum: { $ifNull: ["$report.battle_results.enemy_death", 0] } },
          enemySeverelyWounded: {
            $sum: { $ifNull: ["$report.battle_results.enemy_severely_wounded", 0] },
          },
          enemyWounded: { $sum: { $ifNull: ["$report.battle_results.enemy_wounded", 0] } },
          enemyRemaining: { $sum: { $ifNull: ["$report.battle_results.enemy_remaining", 0] } },
          enemyKillScore: { $sum: { $ifNull: ["$report.battle_results.enemy_kill_score", 0] } },
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

    const battleResults: BattleResultsSummary | undefined = totals ? { total: totals } : undefined;

    return NextResponse.json(
      {
        parentHash: hash,
        items,
        count: items.length,
        ...(battleResults ? { battleResults } : {}),
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
