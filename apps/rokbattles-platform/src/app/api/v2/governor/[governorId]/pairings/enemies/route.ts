import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { parseGovernorId } from "@/lib/governor";
import {
  applyBattleResults,
  type BattleReportDocument,
  buildLoadoutKey,
  buildLoadoutSnapshot,
  createEmptyTotals,
  createMatchStage,
  extractBattleDurationMillis,
  extractEventTimeMillis,
  type LoadoutGranularity,
  normalizeCommanderId,
  resolveDateRange,
} from "@/lib/pairings/shared";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

type EnemyAggregate = {
  enemyPrimaryCommanderId: number;
  enemySecondaryCommanderId: number;
  count: number;
  totals: ReturnType<typeof createEmptyTotals>;
};

type EnemyGranularity = "overall" | LoadoutGranularity;

function parseGranularity(raw: string | null): EnemyGranularity {
  if (raw === "normalized") {
    return "normalized";
  }
  if (raw === "exact") {
    return "exact";
  }
  return "overall";
}

export async function GET(
  req: NextRequest,
  ctx: RouteContext<"/api/v2/governor/[governorId]/pairings/enemies">
) {
  const { governorId: governorParam } = await ctx.params;
  const governorId = parseGovernorId(governorParam);
  if (governorId == null) {
    return NextResponse.json({ error: "Invalid governorId" }, { status: 400 });
  }

  const url = new URL(req.url);
  const primaryParam = url.searchParams.get("primary");
  const secondaryParam = url.searchParams.get("secondary");
  const yearParam = url.searchParams.get("year");
  const startParam = url.searchParams.get("start");
  const endParam = url.searchParams.get("end");
  const granularityParam = url.searchParams.get("granularity");
  const loadoutKey = url.searchParams.get("loadoutKey");
  const primaryCommanderId = primaryParam ? Number(primaryParam) : Number.NaN;
  const secondaryCommanderId = secondaryParam
    ? Number(secondaryParam)
    : Number.NaN;
  if (
    !Number.isFinite(primaryCommanderId) ||
    primaryCommanderId <= 0 ||
    !Number.isFinite(secondaryCommanderId) ||
    secondaryCommanderId < 0
  ) {
    return NextResponse.json({ error: "Invalid pairing" }, { status: 400 });
  }

  const granularity = parseGranularity(granularityParam);
  if (granularity !== "overall" && !loadoutKey) {
    return NextResponse.json({ error: "Missing loadoutKey" }, { status: 400 });
  }

  const authResult = await requireAuthContext();
  if (!authResult.ok) {
    return authResult.response;
  }

  const { db, user } = authResult.context;
  const nowYear = new Date().getUTCFullYear();
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

  const startMillis = range.startMillis;
  const endMillis = range.endMillis;

  const projection: Document = {
    _id: 0,
    "report.metadata.email_time": 1,
    "report.metadata.start_date": 1,
    "report.metadata.end_date": 1,
    "report.self.primary_commander.id": 1,
    "report.self.secondary_commander.id": 1,
    "report.self.equipment": 1,
    "report.self.formation": 1,
    "report.self.armament_buffs": 1,
    "report.self.inscriptions": 1,
    "report.enemy.primary_commander.id": 1,
    "report.enemy.secondary_commander.id": 1,
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
    const matchStage = createMatchStage(governorId, startMillis, endMillis);
    matchStage["report.self.primary_commander.id"] = primaryCommanderId;

    const reports = await db
      .collection<BattleReportDocument>("battleReports")
      .find(matchStage, { projection })
      .toArray();

    const buckets = new Map<
      string,
      {
        enemyPrimaryCommanderId: number;
        enemySecondaryCommanderId: number;
        count: number;
        totals: ReturnType<typeof createEmptyTotals>;
      }
    >();

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

      const selfPrimary = normalizeCommanderId(
        report.self?.primary_commander?.id
      );
      const selfSecondary = normalizeCommanderId(
        report.self?.secondary_commander?.id
      );
      if (
        selfPrimary !== primaryCommanderId ||
        selfSecondary !== secondaryCommanderId
      ) {
        continue;
      }

      if (granularity !== "overall") {
        const snapshot = buildLoadoutSnapshot(report, granularity);
        const key = buildLoadoutKey(snapshot);
        if (key !== loadoutKey) {
          continue;
        }
      }

      const enemyPrimary = normalizeCommanderId(
        report.enemy?.primary_commander?.id
      );
      const enemySecondary = normalizeCommanderId(
        report.enemy?.secondary_commander?.id
      );
      const enemyKey = `${enemyPrimary}:${enemySecondary}`;
      let bucket = buckets.get(enemyKey);
      if (!bucket) {
        bucket = {
          enemyPrimaryCommanderId: enemyPrimary,
          enemySecondaryCommanderId: enemySecondary,
          count: 0,
          totals: createEmptyTotals(),
        };
        buckets.set(enemyKey, bucket);
      }

      bucket.count += 1;
      applyBattleResults(bucket.totals, report.battle_results);
      bucket.totals.battleDuration += extractBattleDurationMillis(report);
    }

    const items: EnemyAggregate[] = Array.from(buckets.values()).map(
      (bucket) => ({
        enemyPrimaryCommanderId: bucket.enemyPrimaryCommanderId,
        enemySecondaryCommanderId: bucket.enemySecondaryCommanderId,
        count: bucket.count,
        totals: bucket.totals,
      })
    );

    items.sort((a, b) => {
      if (b.totals.killScore !== a.totals.killScore) {
        return b.totals.killScore - a.totals.killScore;
      }
      return b.count - a.count;
    });

    return NextResponse.json(
      {
        items,
      },
      {
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  } catch (error) {
    console.error("Failed to load pairing enemies", error);
    return NextResponse.json(
      { error: "Failed to load pairing enemies" },
      {
        status: 500,
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  }
}
