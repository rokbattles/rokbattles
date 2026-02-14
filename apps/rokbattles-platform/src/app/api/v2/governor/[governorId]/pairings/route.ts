import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { parseGovernorId } from "@/lib/governor";
import {
  applyBattleResults,
  createEmptyTotals,
  createMatchStage,
  extractEventTimeMillis,
  flattenPairingBattleEntries,
  type MarchTotals,
  normalizeCommanderId,
  type PairingsBattleMailDocument,
  resolveDateRange,
} from "@/lib/pairings/shared";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

type AggregationBucket = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: MarchTotals;
};

type PairingAggregate = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: AggregationBucket["totals"];
};

function aggregateReports(
  mails: PairingsBattleMailDocument[],
  startMillis: number,
  endMillis: number
) {
  const buckets = new Map<string, AggregationBucket>();

  for (const mail of mails) {
    const eventTime = extractEventTimeMillis(mail);
    if (eventTime == null || eventTime < startMillis || eventTime >= endMillis) {
      continue;
    }

    for (const entry of flattenPairingBattleEntries(mail)) {
      const primaryCommanderId = normalizeCommanderId(entry.selfPrimaryCommanderId);
      if (primaryCommanderId <= 0) {
        continue;
      }

      const secondaryCommanderId = normalizeCommanderId(entry.selfSecondaryCommanderId);
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
      applyBattleResults(bucket.totals, entry.battleResults);
      bucket.totals.battleDuration += entry.battleDurationMillis;
    }
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
  const range = resolveDateRange({ startParam, endParam, fallbackYear: targetYear });

  const claim = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .findOne({ discordId: user.discordId, governorId });
  if (!claim) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const projection: Document = {
    _id: 0,
    "metadata.mail_time": 1,
    "timeline.start_timestamp": 1,
    "sender.commanders.primary.id": 1,
    "sender.commanders.secondary.id": 1,
    "opponents.player_id": 1,
    "opponents.start_tick": 1,
    "opponents.end_tick": 1,
    "opponents.commanders.primary.id": 1,
    "opponents.commanders.secondary.id": 1,
    "opponents.battle_results.sender.kill_points": 1,
    "opponents.battle_results.sender.dead": 1,
    "opponents.battle_results.sender.severely_wounded": 1,
    "opponents.battle_results.sender.slightly_wounded": 1,
    "opponents.battle_results.opponent.kill_points": 1,
    "opponents.battle_results.opponent.dead": 1,
    "opponents.battle_results.opponent.severely_wounded": 1,
    "opponents.battle_results.opponent.slightly_wounded": 1,
  };

  try {
    const reports = await db
      .collection<PairingsBattleMailDocument>("mails_battle")
      .find(createMatchStage(governorId, range.startMillis, range.endMillis), { projection })
      .toArray();

    const buckets = aggregateReports(reports, range.startMillis, range.endMillis);
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
