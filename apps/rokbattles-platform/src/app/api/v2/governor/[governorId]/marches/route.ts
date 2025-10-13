import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import { authenticateRequest } from "@/lib/auth";
import { parseGovernorId } from "@/lib/governor";
import { coerceNumber } from "@/lib/number";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

type BattleReportDocument = {
  report?: {
    metadata?: {
      email_time?: unknown;
      start_date?: unknown;
    };
    self?: {
      primary_commander?: { id?: unknown };
      secondary_commander?: { id?: unknown };
    };
    battle_results?: {
      kill_score?: unknown;
      death?: unknown;
      severely_wounded?: unknown;
      wounded?: unknown;
      enemy_kill_score?: unknown;
      enemy_death?: unknown;
      enemy_severely_wounded?: unknown;
      enemy_wounded?: unknown;
    };
  };
};

type MarchTotals = {
  killScore: number;
  deaths: number;
  severelyWounded: number;
  wounded: number;
  enemyKillScore: number;
  enemyDeaths: number;
  enemySeverelyWounded: number;
  enemyWounded: number;
};

type AggregationBucket = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: MarchTotals;
};

type MarchAggregate = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: AggregationBucket["totals"];
  averageKillScore: number;
  previousTotals: MarchTotals;
  previousCount: number;
};

function createEmptyTotals(): MarchTotals {
  return {
    killScore: 0,
    deaths: 0,
    severelyWounded: 0,
    wounded: 0,
    enemyKillScore: 0,
    enemyDeaths: 0,
    enemySeverelyWounded: 0,
    enemyWounded: 0,
  };
}

function createMatchStage(governorId: number, startMillis: number, endMillis: number): Document {
  const startSeconds = Math.floor(startMillis / 1000);
  const endSeconds = Math.floor(endMillis / 1000);
  const startMicros = Math.floor(startMillis * 1000);
  const endMicros = Math.floor(endMillis * 1000);

  return {
    "report.self.player_id": governorId,
    "report.enemy.player_id": { $nin: [-2, 0] },
    $or: [
      {
        "report.metadata.start_date": {
          $gte: startSeconds,
          $lt: endSeconds,
        },
      },
      {
        "report.metadata.start_date": { $exists: false },
        $or: [
          {
            "report.metadata.email_time": {
              $gte: startMillis,
              $lt: endMillis,
            },
          },
          {
            "report.metadata.email_time": {
              $gte: startMicros,
              $lt: endMicros,
            },
          },
        ],
      },
    ],
  } satisfies Document;
}

function aggregateReports(reports: BattleReportDocument[], startMillis: number, endMillis: number) {
  const buckets = new Map<string, AggregationBucket>();

  for (const doc of reports) {
    const report = doc.report;
    if (!report) {
      continue;
    }

    const eventTime = extractEventTimeMillis(report);
    if (eventTime == null || eventTime < startMillis || eventTime >= endMillis) {
      continue;
    }

    const primaryCommanderId = Math.trunc(coerceNumber(report.self?.primary_commander?.id));
    if (primaryCommanderId <= 0) {
      continue;
    }

    const secondaryCommanderId = Math.trunc(coerceNumber(report.self?.secondary_commander?.id));
    const key = `${primaryCommanderId}:${secondaryCommanderId}`;

    let bucket = buckets.get(key);
    if (!bucket) {
      bucket = {
        primaryCommanderId,
        secondaryCommanderId,
        count: 0,
        totals: createEmptyTotals(),
      };
      buckets.set(key, bucket);
    }

    const battleResults = report.battle_results;
    bucket.count += 1;

    if (battleResults) {
      const killScore = coerceNumber(battleResults.kill_score);
      const deaths = coerceNumber(battleResults.death);
      const severelyWounded = coerceNumber(battleResults.severely_wounded);
      const wounded = coerceNumber(battleResults.wounded);
      const enemyKillScore = coerceNumber(battleResults.enemy_kill_score);
      const enemyDeaths = coerceNumber(battleResults.enemy_death);
      const enemySeverelyWounded = coerceNumber(battleResults.enemy_severely_wounded);
      const enemyWounded = coerceNumber(battleResults.enemy_wounded);

      bucket.totals.killScore += killScore;
      bucket.totals.deaths += deaths;
      bucket.totals.severelyWounded += severelyWounded;
      bucket.totals.wounded += wounded;
      bucket.totals.enemyKillScore += enemyKillScore;
      bucket.totals.enemyDeaths += enemyDeaths;
      bucket.totals.enemySeverelyWounded += enemySeverelyWounded;
      bucket.totals.enemyWounded += enemyWounded;
    }
  }

  return buckets;
}

function extractEventTimeMillis(report: BattleReportDocument["report"]): number | null {
  const rawMetadata = report?.metadata;
  if (!rawMetadata) {
    return null;
  }

  const emailTime = coerceNumber(rawMetadata.email_time);
  if (emailTime > 0) {
    if (emailTime >= 1e14) {
      return emailTime / 1000;
    }
    if (emailTime >= 1e12) {
      return emailTime;
    }
    return emailTime * 1000;
  }

  const startDate = coerceNumber(rawMetadata.start_date);
  if (startDate > 0) {
    return startDate < 1e12 ? startDate * 1000 : startDate;
  }

  return null;
}

export async function GET(
  _req: NextRequest,
  ctx: RouteContext<"/api/v2/governor/[governorId]/marches">
) {
  const { governorId: governorParam } = await ctx.params;
  const governorId = parseGovernorId(governorParam);
  if (!governorId) {
    return NextResponse.json({ error: "Invalid governorId" }, { status: 400 });
  }

  const authResult = await authenticateRequest();
  if (authResult.ok === false) {
    if (authResult.reason === "session-expired") {
      return NextResponse.json({ error: "Session expired" }, { status: 401 });
    }

    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const { db, user } = authResult.context;
  const now = new Date();

  const claim = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .findOne({ discordId: user.discordId, governorId });
  if (!claim) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const startOfMonth = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), 1));
  const startMillis = startOfMonth.getTime();
  const nextMonth = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() + 1, 1));
  const endMillis = nextMonth.getTime();
  const previousStartOfMonth = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() - 1, 1));
  const previousStartMillis = previousStartOfMonth.getTime();
  const previousEndMillis = startMillis;

  const projection: Document = {
    _id: 0,
    "report.metadata.email_time": 1,
    "report.metadata.start_date": 1,
    "report.self.primary_commander.id": 1,
    "report.self.secondary_commander.id": 1,
    "report.battle_results.kill_score": 1,
    "report.battle_results.death": 1,
    "report.battle_results.severely_wounded": 1,
    "report.battle_results.wounded": 1,
    "report.battle_results.enemy_kill_score": 1,
    "report.battle_results.enemy_death": 1,
    "report.battle_results.enemy_severely_wounded": 1,
    "report.battle_results.enemy_wounded": 1,
  };

  try {
    const [currentReports, previousReports] = await Promise.all([
      db
        .collection<BattleReportDocument>("battleReports")
        .find(createMatchStage(governorId, startMillis, endMillis), { projection })
        .toArray(),
      db
        .collection<BattleReportDocument>("battleReports")
        .find(createMatchStage(governorId, previousStartMillis, previousEndMillis), { projection })
        .toArray(),
    ]);

    const currentBuckets = aggregateReports(currentReports, startMillis, endMillis);
    const previousBuckets = aggregateReports(
      previousReports,
      previousStartMillis,
      previousEndMillis
    );

    const items: MarchAggregate[] = [];
    for (const bucket of currentBuckets.values()) {
      const key = `${bucket.primaryCommanderId}:${bucket.secondaryCommanderId}`;
      const previous = previousBuckets.get(key);
      const previousTotals = previous ? { ...previous.totals } : createEmptyTotals();
      const previousCount = previous?.count ?? 0;

      items.push({
        primaryCommanderId: bucket.primaryCommanderId,
        secondaryCommanderId: bucket.secondaryCommanderId,
        count: bucket.count,
        totals: { ...bucket.totals },
        averageKillScore: bucket.count > 0 ? bucket.totals.killScore / bucket.count : 0,
        previousTotals,
        previousCount,
      });
    }

    items.sort((a, b) => {
      if (b.totals.killScore !== a.totals.killScore) {
        return b.totals.killScore - a.totals.killScore;
      }
      return b.count - a.count;
    });

    return NextResponse.json(
      {
        governorId,
        period: {
          start: startOfMonth.toISOString(),
          end: new Date(endMillis - 1).toISOString(),
        },
        count: items.length,
        items,
      },
      {
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  } catch (error) {
    console.error("Failed to load governor marches", error);
    return NextResponse.json(
      { error: "Failed to load governor marches" },
      {
        status: 500,
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  }
}
