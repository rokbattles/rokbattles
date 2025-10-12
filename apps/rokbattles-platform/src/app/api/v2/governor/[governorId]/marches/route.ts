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

type AggregationBucket = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: {
    killScore: number;
    deaths: number;
    severelyWounded: number;
    wounded: number;
    enemyKillScore: number;
    enemyDeaths: number;
    enemySeverelyWounded: number;
    enemyWounded: number;
  };
  killScores: number[];
};

type MarchAggregate = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: AggregationBucket["totals"];
  averageKillScore: number;
  killScorePercentiles: {
    p10: number;
    p25: number;
    p50: number;
    p75: number;
    p90: number;
  };
};

function computePercentile(sortedValues: number[], percentile: number): number {
  if (sortedValues.length === 0) {
    return 0;
  }

  if (sortedValues.length === 1 || percentile <= 0) {
    return sortedValues[0];
  }

  if (percentile >= 1) {
    return sortedValues[sortedValues.length - 1];
  }

  const rank = (sortedValues.length - 1) * percentile;
  const lowerIndex = Math.floor(rank);
  const upperIndex = Math.ceil(rank);

  if (lowerIndex === upperIndex) {
    return sortedValues[lowerIndex];
  }

  const weight = rank - lowerIndex;
  const lowerValue = sortedValues[lowerIndex];
  const upperValue = sortedValues[upperIndex];

  return lowerValue + (upperValue - lowerValue) * weight;
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

  const startSeconds = Math.floor(startMillis / 1000);
  const endSeconds = Math.floor(endMillis / 1000);
  const startMicros = Math.floor(startMillis * 1000);
  const endMicros = Math.floor(endMillis * 1000);

  const matchStage: Document = {
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
  };

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
    const reports = await db
      .collection<BattleReportDocument>("battleReports")
      .find(matchStage, { projection })
      .toArray();

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
          totals: {
            killScore: 0,
            deaths: 0,
            severelyWounded: 0,
            wounded: 0,
            enemyKillScore: 0,
            enemyDeaths: 0,
            enemySeverelyWounded: 0,
            enemyWounded: 0,
          },
          killScores: [],
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
        bucket.killScores.push(killScore);
      }
    }

    const items: MarchAggregate[] = [];
    for (const bucket of buckets.values()) {
      const sortedKillScores = bucket.killScores.slice().sort((a, b) => a - b);
      const p10 = computePercentile(sortedKillScores, 0.1);
      const p25 = computePercentile(sortedKillScores, 0.25);
      const p50 = computePercentile(sortedKillScores, 0.5);
      const p75 = computePercentile(sortedKillScores, 0.75);
      const p90 = computePercentile(sortedKillScores, 0.9);

      items.push({
        primaryCommanderId: bucket.primaryCommanderId,
        secondaryCommanderId: bucket.secondaryCommanderId,
        count: bucket.count,
        totals: bucket.totals,
        averageKillScore: bucket.count > 0 ? bucket.totals.killScore / bucket.count : 0,
        killScorePercentiles: {
          p10: p10,
          p25: p25,
          p50: p50,
          p75: p75,
          p90: p90,
        },
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
