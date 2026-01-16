import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { parseGovernorId } from "@/lib/governor";
import {
  applyBattleResults,
  type BattleReportDocument,
  createEmptyTotals,
  createMatchStage,
  extractBattleDurationMillis,
  extractEventTimeMillis,
  type MarchTotals,
  normalizeCommanderId,
  resolveDateRange,
} from "@/lib/pairings/shared";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

interface AggregationBucket {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: MarchTotals;
}

interface PairingAggregate {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: AggregationBucket["totals"];
}

function aggregateReports(
  reports: BattleReportDocument[],
  startMillis: number,
  endMillis: number
) {
  const buckets = new Map<string, AggregationBucket>();

  for (const doc of reports) {
    const report = doc.report;
    if (!report) {
      continue;
    }

    const eventTime = extractEventTimeMillis(report);
    if (
      eventTime == null ||
      eventTime < startMillis ||
      eventTime >= endMillis
    ) {
      continue;
    }

    const primaryCommanderId = normalizeCommanderId(
      report.self?.primary_commander?.id
    );
    if (primaryCommanderId <= 0) {
      continue;
    }

    const secondaryCommanderId = normalizeCommanderId(
      report.self?.secondary_commander?.id
    );
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

    bucket.count += 1;
    applyBattleResults(bucket.totals, report.battle_results);
    bucket.totals.battleDuration += extractBattleDurationMillis(report);
  }

  return buckets;
}

export async function GET(
  req: NextRequest,
  ctx: RouteContext<"/api/v2/governor/[governorId]/pairings">
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
  const nowYear = new Date().getUTCFullYear();
  const url = new URL(req.url);
  const yearParam = url.searchParams.get("year");
  const startParam = url.searchParams.get("start");
  const endParam = url.searchParams.get("end");
  const parsedYear = yearParam ? Number(yearParam) : Number.NaN;
  const targetYear = Number.isFinite(parsedYear) ? parsedYear : nowYear;
  const range = resolveDateRange({
    startParam,
    endParam,
    fallbackYear: targetYear,
  });

  const claim = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .findOne({ discordId: user.discordId, governorId });
  if (!claim) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

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
    const reports = await db
      .collection<BattleReportDocument>("battleReports")
      .find(createMatchStage(governorId, range.startMillis, range.endMillis), {
        projection,
      })
      .toArray();

    const buckets = aggregateReports(
      reports,
      range.startMillis,
      range.endMillis
    );
    const items: PairingAggregate[] = [];

    for (const bucket of buckets.values()) {
      items.push({
        primaryCommanderId: bucket.primaryCommanderId,
        secondaryCommanderId: bucket.secondaryCommanderId,
        count: bucket.count,
        totals: bucket.totals,
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
        year: range.year,
        items,
      },
      {
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  } catch (error) {
    console.error("Failed to load governor pairings", error);
    return NextResponse.json(
      { error: "Failed to load governor pairings" },
      {
        status: 500,
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  }
}
