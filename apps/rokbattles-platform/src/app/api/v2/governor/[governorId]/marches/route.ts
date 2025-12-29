import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { normalizeTimestampMillis } from "@/lib/datetime";
import { parseGovernorId } from "@/lib/governor";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

type BattleReportDocument = {
  report?: {
    metadata?: {
      email_time?: unknown;
      start_date?: unknown;
      end_date?: unknown;
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
  dps: number;
  sps: number;
  tps: number;
  battleDuration: number;
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
  monthly: MonthlyAggregate[];
};

type MonthlyAggregate = {
  monthKey: string;
  count: number;
  totals: MarchTotals;
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
    dps: 0,
    sps: 0,
    tps: 0,
    battleDuration: 0,
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

    const primaryCommanderId = Number(report.self?.primary_commander?.id);
    const secondaryCommanderId = Number(report.self?.secondary_commander?.id);
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
      const killScore = Number(battleResults.kill_score);
      const deaths = Number(battleResults.death);
      const severelyWounded = Number(battleResults.severely_wounded);
      const wounded = Number(battleResults.wounded);
      const enemyKillScore = Number(battleResults.enemy_kill_score);
      const enemyDeaths = Number(battleResults.enemy_death);
      const enemySeverelyWounded = Number(battleResults.enemy_severely_wounded);
      const enemyWounded = Number(battleResults.enemy_wounded);

      bucket.totals.killScore += killScore;
      bucket.totals.deaths += deaths;
      bucket.totals.severelyWounded += severelyWounded;
      bucket.totals.wounded += wounded;
      bucket.totals.enemyKillScore += enemyKillScore;
      bucket.totals.enemyDeaths += enemyDeaths;
      bucket.totals.enemySeverelyWounded += enemySeverelyWounded;
      bucket.totals.enemyWounded += enemyWounded;
      bucket.totals.dps += enemyWounded + enemySeverelyWounded;
      bucket.totals.sps += enemySeverelyWounded;
      bucket.totals.tps += severelyWounded;
    }

    const battleDurationMillis = extractBattleDurationMillis(report);
    bucket.totals.battleDuration += battleDurationMillis;
  }

  return buckets;
}

function extractEventTimeMillis(report: BattleReportDocument["report"]): number | null {
  const rawMetadata = report?.metadata;
  if (!rawMetadata) {
    return null;
  }

  const emailTime = normalizeTimestampMillis(rawMetadata.email_time);
  if (emailTime != null) {
    return emailTime;
  }

  const startDate = normalizeTimestampMillis(rawMetadata.start_date);
  if (startDate != null) {
    return startDate;
  }

  return null;
}

function extractBattleDurationMillis(report: BattleReportDocument["report"]): number {
  const rawMetadata = report?.metadata;
  if (!rawMetadata) {
    return 0;
  }

  const start = normalizeTimestampMillis(rawMetadata.start_date);
  const end = normalizeTimestampMillis(rawMetadata.end_date);

  if (start == null || end == null) {
    return 0;
  }

  return end - start;
}

function createMonthKey(year: number, monthIndex: number) {
  return `${year}-${String(monthIndex + 1).padStart(2, "0")}`;
}

function buildMonthsForYear(year: number) {
  return Array.from({ length: 12 }, (_, monthIndex) => {
    const start = new Date(Date.UTC(year, monthIndex, 1));
    const end = new Date(Date.UTC(year, monthIndex + 1, 1));

    return {
      key: createMonthKey(year, monthIndex),
      startMillis: start.getTime(),
      endMillis: end.getTime(),
      monthIndex,
    };
  });
}

function addTotals(target: MarchTotals, increment: MarchTotals) {
  target.killScore += increment.killScore;
  target.deaths += increment.deaths;
  target.severelyWounded += increment.severelyWounded;
  target.wounded += increment.wounded;
  target.enemyKillScore += increment.enemyKillScore;
  target.enemyDeaths += increment.enemyDeaths;
  target.enemySeverelyWounded += increment.enemySeverelyWounded;
  target.enemyWounded += increment.enemyWounded;
  target.dps += increment.dps;
  target.sps += increment.sps;
  target.tps += increment.tps;
  target.battleDuration += increment.battleDuration;
}

export async function GET(
  req: NextRequest,
  ctx: RouteContext<"/api/v2/governor/[governorId]/marches">
) {
  const { governorId: governorParam } = await ctx.params;
  const governorId = parseGovernorId(governorParam);
  if (governorId == null) {
    return NextResponse.json({ error: "Invalid governorId" }, { status: 400 });
  }

  const authResult = await requireAuthContext();
  if (!authResult.ok) {
    return authResult.response;
  }

  const { db, user } = authResult.context;
  const now = new Date();
  const nowYear = now.getUTCFullYear();
  const nowMonth = now.getUTCMonth();
  const url = new URL(req.url);
  const yearParam = url.searchParams.get("year");
  const parsedYear = yearParam ? Number(yearParam) : Number.NaN;
  const targetYear = Number.isFinite(parsedYear) ? parsedYear : nowYear;

  const claim = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .findOne({ discordId: user.discordId, governorId });
  if (!claim) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const months = buildMonthsForYear(targetYear);

  const projection: Document = {
    _id: 0,
    "report.metadata.email_time": 1,
    "report.metadata.start_date": 1,
    "report.metadata.end_date": 1,
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
    const monthlyBuckets = await Promise.all(
      months.map((month) => {
        const isFutureMonth =
          targetYear > nowYear || (targetYear === nowYear && month.monthIndex > nowMonth);
        if (isFutureMonth) {
          return Promise.resolve(new Map<string, AggregationBucket>());
        }

        return db
          .collection<BattleReportDocument>("battleReports")
          .find(createMatchStage(governorId, month.startMillis, month.endMillis), { projection })
          .toArray()
          .then((reports) => aggregateReports(reports, month.startMillis, month.endMillis));
      })
    );

    const merged = new Map<
      string,
      {
        primaryCommanderId: number;
        secondaryCommanderId: number;
        monthly: Map<string, MonthlyAggregate>;
      }
    >();

    months.forEach((month, index) => {
      const buckets = monthlyBuckets[index];
      for (const bucket of buckets.values()) {
        const key = `${bucket.primaryCommanderId}:${bucket.secondaryCommanderId}`;
        let entry = merged.get(key);
        if (!entry) {
          entry = {
            primaryCommanderId: bucket.primaryCommanderId,
            secondaryCommanderId: bucket.secondaryCommanderId,
            monthly: new Map<string, MonthlyAggregate>(),
          };
          merged.set(key, entry);
        }

        entry.monthly.set(month.key, {
          monthKey: month.key,
          count: bucket.count,
          totals: { ...bucket.totals },
        });
      }
    });

    const items: MarchAggregate[] = [];

    for (const entry of merged.values()) {
      const monthly: MonthlyAggregate[] = months.map((month) => {
        const found = entry.monthly.get(month.key);
        if (found) {
          return found;
        }

        return {
          monthKey: month.key,
          count: 0,
          totals: createEmptyTotals(),
        };
      });

      const totals = createEmptyTotals();
      let count = 0;
      for (const month of monthly) {
        count += month.count;
        addTotals(totals, month.totals);
      }

      items.push({
        primaryCommanderId: entry.primaryCommanderId,
        secondaryCommanderId: entry.secondaryCommanderId,
        count,
        totals,
        averageKillScore: count > 0 ? totals.killScore / count : 0,
        previousTotals: createEmptyTotals(),
        previousCount: 0,
        monthly,
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
        year: targetYear,
        governorId,
        period: {
          start: new Date(Date.UTC(targetYear, 0, 1, 0, 0, 0, 0)).toISOString(),
          end: new Date(Date.UTC(targetYear, 11, 31, 23, 59, 59, 999)).toISOString(),
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
