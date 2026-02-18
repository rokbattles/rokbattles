import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { parseGovernorId } from "@/lib/governor";
import {
  applyBattleResults,
  buildLoadoutKey,
  buildLoadoutSnapshot,
  createEmptyTotals,
  createMatchStage,
  extractEventTimeMillis,
  flattenPairingBattleEntries,
  type LoadoutGranularity,
  type LoadoutSnapshot,
  type PairingsBattleMailDocument,
  resolveDateRange,
} from "@/lib/pairings/shared";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

type LoadoutAggregate = {
  key: string;
  count: number;
  totals: ReturnType<typeof createEmptyTotals>;
  loadout: LoadoutSnapshot;
};

function parseGranularity(raw: string | null): LoadoutGranularity {
  return raw === "normalized" ? "normalized" : "exact";
}

export async function GET(
  req: NextRequest,
  ctx: RouteContext<"/api/v2/governor/[governorId]/pairings/loadouts">
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
  const primaryCommanderId = primaryParam ? Number(primaryParam) : Number.NaN;
  const secondaryCommanderId = secondaryParam ? Number(secondaryParam) : Number.NaN;
  if (
    !Number.isFinite(primaryCommanderId) ||
    primaryCommanderId <= 0 ||
    !Number.isFinite(secondaryCommanderId) ||
    secondaryCommanderId < 0
  ) {
    return NextResponse.json({ error: "Invalid pairing" }, { status: 400 });
  }

  const authResult = await requireAuthContext();
  if (!authResult.ok) {
    return authResult.response;
  }

  const { db, user } = authResult.context;
  const nowYear = new Date().getUTCFullYear();
  const parsedYear = yearParam ? Number(yearParam) : Number.NaN;
  const targetYear = Number.isFinite(parsedYear) ? parsedYear : nowYear;
  const granularity = parseGranularity(granularityParam);
  const range = resolveDateRange({ startParam, endParam, fallbackYear: targetYear });

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
    "metadata.mail_time": 1,
    "timeline.start_timestamp": 1,
    "sender.commanders.primary.id": 1,
    "sender.commanders.secondary.id": 1,
    "sender.commanders.primary.equipment": 1,
    "sender.commanders.primary.formation": 1,
    "sender.commanders.primary.armaments": 1,
    "sender.commanders.secondary.armaments": 1,
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
    const matchStage = createMatchStage(governorId, startMillis, endMillis);
    matchStage["sender.commanders.primary.id"] = primaryCommanderId;

    const reports = await db
      .collection<PairingsBattleMailDocument>("mails_battle")
      .find(matchStage, { projection })
      .toArray();

    const buckets = new Map<
      string,
      { loadout: LoadoutSnapshot; count: number; totals: ReturnType<typeof createEmptyTotals> }
    >();

    for (const mail of reports) {
      const eventTime = extractEventTimeMillis(mail);
      if (eventTime == null || eventTime < startMillis || eventTime >= endMillis) {
        continue;
      }

      const loadout = buildLoadoutSnapshot(mail, granularity);
      const key = buildLoadoutKey(loadout);

      for (const entry of flattenPairingBattleEntries(mail)) {
        if (
          entry.selfPrimaryCommanderId !== primaryCommanderId ||
          entry.selfSecondaryCommanderId !== secondaryCommanderId
        ) {
          continue;
        }

        let bucket = buckets.get(key);
        if (!bucket) {
          bucket = {
            loadout,
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

    const items: LoadoutAggregate[] = Array.from(buckets.entries()).map(([key, bucket]) => ({
      key,
      count: bucket.count,
      totals: bucket.totals,
      loadout: bucket.loadout,
    }));

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
    console.error("Failed to load pairing loadouts", error);
    return NextResponse.json(
      { error: "Failed to load pairing loadouts" },
      {
        status: 500,
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  }
}
