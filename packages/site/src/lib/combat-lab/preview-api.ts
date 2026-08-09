import "server-only";

import { notFound } from "next/navigation";
import {
  type CombatLabPreviewData,
  type CombatLabPreviewRangeKey,
  type CombatLabPreviewScenario,
  type CombatLabPreviewScenarioKey,
  combatLabPreviewRangeKeys,
  combatLabPreviewScenarioKeys,
} from "@/lib/combat-lab/preview-types";
import { getCommanderName } from "@/lib/commander";

type WireDrastc = Omit<NonNullable<CombatLabPreviewData["drastc"]>, "confidence"> & {
  confidence: {
    score: number;
    uniqueGovernors: number;
    effectiveGovernors: number;
  };
};

type CombatLabV2Wire = {
  generatedAtMs: number;
  pairing: {
    primaryCommanderId: number;
    secondaryCommanderId: number;
  };
  drastc?: WireDrastc | null;
  summaries: number[][];
  performance: number[][];
  loadouts: Array<Array<number | number[]>>;
  armamentMaximums: Record<string, number>;
};

const DAY_MS = 24 * 60 * 60 * 1000;
const rangeDays: Record<CombatLabPreviewRangeKey, number> = {
  "1y": 365,
  "6m": 183,
  "1m": 30,
  "7d": 7,
};

export async function fetchCombatLabPreview(options: {
  primary: number;
  secondary: number;
  locale?: string;
}): Promise<CombatLabPreviewData> {
  const params = new URLSearchParams({
    primary: options.primary.toString(),
    secondary: options.secondary.toString(),
  });
  const apiUrl = process.env.API_URL || "http://localhost:8001";
  const response = await fetch(`${apiUrl.replace(/\/$/, "")}/v2/global/combat-lab?${params}`, {
    cache: "no-store",
  });
  if (response.status === 404) notFound();
  if (!response.ok) {
    throw new Error(`Could not load Combat Lab data (${response.status})`);
  }
  const wire = (await response.json()) as CombatLabV2Wire;
  return expandWireData(wire, options.locale);
}

function expandWireData(wire: CombatLabV2Wire, locale?: string): CombatLabPreviewData {
  const ranges = Object.fromEntries(
    combatLabPreviewRangeKeys.map((range) => [
      range,
      {
        scenarios: Object.fromEntries(
          combatLabPreviewScenarioKeys.map((scenario) => [scenario, emptyScenario()])
        ),
      },
    ])
  ) as CombatLabPreviewData["ranges"];
  const cutoffs = Object.fromEntries(
    combatLabPreviewRangeKeys.map((range) => [
      range,
      wire.generatedAtMs - rangeDays[range] * DAY_MS,
    ])
  ) as Record<CombatLabPreviewRangeKey, number>;

  for (const tuple of wire.summaries ?? []) {
    const range = combatLabPreviewRangeKeys[tuple[0]];
    const scenario = combatLabPreviewScenarioKeys[tuple[1]];
    const output = range && scenario ? ranges[range]?.scenarios?.[scenario] : undefined;
    if (!output) continue;
    const battles = finite(tuple[2]);
    const killPointsGained = finite(tuple[4]);
    const killPointsLost = finite(tuple[5]);
    const durationMs = finite(tuple[8]);
    const rateDurationMs = finite(tuple[9]);
    output.summary = {
      battles,
      uniqueGovernors: finite(tuple[3]),
      killPointsGained,
      killPointsLost,
      severelyWoundedInflicted: finite(tuple[6]),
      severelyWoundedTaken: finite(tuple[7]),
      averageBattleDurationSeconds: divide(durationMs, battles) / 1000,
      weightedTradePercent: tradePercent(killPointsGained, killPointsLost),
      dps: perSecond(finite(tuple[10]), rateDurationMs),
      sps: perSecond(finite(tuple[6]), rateDurationMs),
      tps: perSecond(finite(tuple[7]), rateDurationMs),
      hps: perSecond(finite(tuple[11]), rateDurationMs),
    };
  }

  for (const tuple of wire.performance ?? []) {
    const day = finite(tuple[0]);
    const scenario = combatLabPreviewScenarioKeys[tuple[1]];
    if (!scenario) continue;
    forEachMatchingRange(day, cutoffs, (range) => {
      const output = ranges[range]?.scenarios?.[scenario];
      if (!output) return;
      const rateDurationMs = finite(tuple[8]);
      output.trends.push({
        bucketStartMs: day,
        battles: finite(tuple[2]),
        killPointsGained: finite(tuple[3]),
        killPointsLost: finite(tuple[4]),
        dps: perSecond(finite(tuple[9]), rateDurationMs),
        sps: perSecond(finite(tuple[5]), rateDurationMs),
        tps: perSecond(finite(tuple[6]), rateDurationMs),
        hps: perSecond(finite(tuple[10]), rateDurationMs),
      });
    });
  }

  const accessoryAggregates = new Map<
    string,
    {
      sampleSize: number;
      pairs: Map<string, { firstItemId: number; secondItemId: number; count: number }>;
    }
  >();
  for (const tuple of wire.loadouts ?? []) {
    const kind = finite(tuple[0] as number);
    const day = finite(tuple[1] as number);
    const scenario = combatLabPreviewScenarioKeys[finite(tuple[2] as number)];
    if (!scenario) continue;
    forEachMatchingRange(day, cutoffs, (range) => {
      const output = ranges[range]?.scenarios?.[scenario];
      if (!output) return;
      if (kind === 0) addFormation(output, tuple);
      if (kind === 1) addArmament(output, tuple, wire.armamentMaximums ?? {});
      if (kind === 2) addEquipment(output, tuple);
      if (kind === 3) addAccessory(accessoryAggregates, range, scenario, tuple);
    });
  }
  for (const range of combatLabPreviewRangeKeys) {
    for (const scenario of combatLabPreviewScenarioKeys) {
      const output = ranges[range]?.scenarios?.[scenario];
      output?.trends.sort((left, right) => left.bucketStartMs - right.bucketStartMs);
      output?.formationUsage.sort((left, right) => left.bucketStartMs - right.bucketStartMs);
      for (const slot of output?.loadouts.armaments.slots ?? []) {
        slot.points.sort((left, right) => left.bucketStartMs - right.bucketStartMs);
      }
      for (const slot of output?.loadouts.equipment.slots ?? []) {
        slot.points.sort((left, right) => left.bucketStartMs - right.bucketStartMs);
      }
      const aggregate = accessoryAggregates.get(`${range}:${scenario}`);
      const equipment = output?.loadouts.equipment;
      if (!aggregate || !equipment) continue;
      equipment.accessoryPairings = {
        sampleSize: aggregate.sampleSize,
        pairings: [...aggregate.pairs.values()].sort((left, right) => right.count - left.count),
      };
    }
  }

  return {
    generatedAtMs: wire.generatedAtMs,
    pairing: {
      primaryCommanderId: wire.pairing.primaryCommanderId,
      primaryCommanderName:
        getCommanderName(wire.pairing.primaryCommanderId, locale) ??
        wire.pairing.primaryCommanderId.toString(),
      secondaryCommanderId: wire.pairing.secondaryCommanderId,
      secondaryCommanderName:
        getCommanderName(wire.pairing.secondaryCommanderId, locale) ??
        wire.pairing.secondaryCommanderId.toString(),
    },
    drastc: wire.drastc
      ? {
          ...wire.drastc,
          confidence: {
            score: wire.drastc.confidence.score,
            unique_governors: wire.drastc.confidence.uniqueGovernors,
            effective_governors: wire.drastc.confidence.effectiveGovernors,
          },
        }
      : null,
    ranges,
  };
}

function emptyScenario(): CombatLabPreviewScenario {
  return {
    summary: {
      battles: 0,
      uniqueGovernors: 0,
      killPointsGained: 0,
      killPointsLost: 0,
      severelyWoundedInflicted: 0,
      severelyWoundedTaken: 0,
      averageBattleDurationSeconds: 0,
      weightedTradePercent: 0,
      dps: 0,
      sps: 0,
      tps: 0,
      hps: 0,
    },
    trends: [],
    formationUsage: [],
    loadouts: {
      armaments: { slots: [1, 2, 3, 4].map((slot) => ({ slot, points: [] })) },
      equipment: {
        slots: [1, 2, 3, 4, 5, 6, 7].map((slot) => ({ slot, points: [] })),
        accessoryPairings: { sampleSize: 0, pairings: [] },
      },
    },
  };
}

function addFormation(output: CombatLabPreviewScenario, tuple: Array<number | number[]>) {
  const formations = pairValues(tuple.slice(4) as number[]).map(([id, count]) => ({ id, count }));
  output.formationUsage.push({
    bucketStartMs: finite(tuple[1] as number),
    sampleSize: finite(tuple[3] as number),
    formations,
  });
}

function addArmament(
  output: CombatLabPreviewScenario,
  tuple: Array<number | number[]>,
  maximums: Record<string, number>
) {
  const slotId = finite(tuple[3] as number);
  const sampleSize = finite(tuple[4] as number);
  const slot = output.loadouts.armaments.slots.find((item) => item.slot === slotId);
  if (!slot) return;
  const metric = (index: number) => {
    const count = finite(tuple[index] as number);
    return { count, percent: divide(count, sampleSize) * 100 };
  };
  const buffs = tuple[11];
  slot.points.push({
    bucketStartMs: finite(tuple[1] as number),
    sampleSize,
    inscriptions: {
      special: metric(5),
      rare: metric(6),
      common: metric(7),
      specialCommon: metric(8),
      rareCommon: metric(9),
      commonCommon: metric(10),
    },
    buffs: groups(Array.isArray(buffs) ? buffs : [], 4).map(
      ([id, observations, totalRoll, maxRollCount]) => ({
        id,
        observations,
        usagePercent: divide(observations, sampleSize) * 100,
        averageRoll: divide(totalRoll, observations),
        maximumRoll: finite(maximums[id.toString()]),
        maxRollCount,
        maxRollPercent: divide(maxRollCount, observations) * 100,
      })
    ),
  });
}

function addEquipment(output: CombatLabPreviewScenario, tuple: Array<number | number[]>) {
  const slotId = finite(tuple[3] as number);
  const slot = output.loadouts.equipment.slots.find((item) => item.slot === slotId);
  if (!slot) return;
  const legendaryCount = finite(tuple[4] as number);
  const nonLegendaryCount = finite(tuple[5] as number);
  slot.points.push({
    bucketStartMs: finite(tuple[1] as number),
    sampleSize: legendaryCount + nonLegendaryCount,
    legendaryCount,
    nonLegendaryCount,
    specialTalentCount: finite(tuple[6] as number),
    noSpecialTalentCount: finite(tuple[7] as number),
    items: pairValues(Array.isArray(tuple[8]) ? tuple[8] : []).map(([id, count]) => ({
      id,
      count,
    })),
    iconicLevels: pairValues(Array.isArray(tuple[9]) ? tuple[9] : []).map(([level, count]) => ({
      level,
      count,
    })),
  });
}

function addAccessory(
  aggregates: Map<
    string,
    {
      sampleSize: number;
      pairs: Map<string, { firstItemId: number; secondItemId: number; count: number }>;
    }
  >,
  range: CombatLabPreviewRangeKey,
  scenario: CombatLabPreviewScenarioKey,
  tuple: Array<number | number[]>
) {
  const key = `${range}:${scenario}`;
  const aggregate = aggregates.get(key) ?? { sampleSize: 0, pairs: new Map() };
  aggregate.sampleSize += finite(tuple[3] as number);
  const values = Array.isArray(tuple[4]) ? tuple[4] : [];
  for (const [firstItemId, secondItemId, count] of groups(values, 3)) {
    const pairKey = `${firstItemId}:${secondItemId}`;
    const current = aggregate.pairs.get(pairKey) ?? { firstItemId, secondItemId, count: 0 };
    current.count += count;
    aggregate.pairs.set(pairKey, current);
  }
  aggregates.set(key, aggregate);
}

function forEachMatchingRange(
  day: number,
  cutoffs: Record<CombatLabPreviewRangeKey, number>,
  visit: (range: CombatLabPreviewRangeKey) => void
) {
  for (const range of combatLabPreviewRangeKeys) if (day >= cutoffs[range]) visit(range);
}

function groups(values: number[], size: number): number[][] {
  const output: number[][] = [];
  for (let index = 0; index + size <= values.length; index += size) {
    output.push(values.slice(index, index + size).map(finite));
  }
  return output;
}

function pairValues(values: number[]): number[][] {
  return groups(values, 2);
}

function finite(value: number | undefined): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function divide(numerator: number, denominator: number): number {
  return denominator > 0 ? numerator / denominator : 0;
}

function perSecond(total: number, durationMs: number): number {
  return divide(total, durationMs / 1000);
}

function tradePercent(gained: number, lost: number): number {
  if (gained === lost) return 100;
  return lost > 0 ? (gained / lost) * 100 : 0;
}
