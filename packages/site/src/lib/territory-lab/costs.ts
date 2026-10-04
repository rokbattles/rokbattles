import type { Building, BuildingCost, BuildingCostSchedule, BuildingKind } from "./types";

export type CostTotals = Required<BuildingCost> & { unknown: number };

export type BuildingCostEntry = {
  building: Building;
  number: number;
  cost: BuildingCost | null;
};

function costAt(schedule: BuildingCostSchedule, kind: BuildingKind, number: number) {
  return schedule[kind]?.find((tier) => number >= tier.from && number <= tier.to)?.cost ?? null;
}

export function buildingCostBreakdown(
  buildings: Building[],
  schedule: BuildingCostSchedule,
  allianceId: string
): BuildingCostEntry[] {
  const counts = new Map<BuildingKind, number>();
  const entries: BuildingCostEntry[] = [];
  for (const building of buildings) {
    if (building.allianceId !== allianceId) continue;
    const number = (counts.get(building.kind) ?? 0) + 1;
    counts.set(building.kind, number);
    entries.push({
      building,
      number,
      cost: costAt(schedule, building.kind, number),
    });
  }
  return entries;
}

export function calculateCostTotals(
  buildings: Building[],
  schedule: BuildingCostSchedule,
  allianceId: string
): CostTotals {
  const totals: CostTotals = {
    credits: 0,
    food: 0,
    wood: 0,
    stone: 0,
    gold: 0,
    crystal: 0,
    unknown: 0,
  };
  for (const { cost } of buildingCostBreakdown(buildings, schedule, allianceId)) {
    if (!cost) {
      totals.unknown += 1;
      continue;
    }
    totals.credits += cost.credits ?? 0;
    totals.food += cost.food ?? 0;
    totals.wood += cost.wood ?? 0;
    totals.stone += cost.stone ?? 0;
    totals.gold += cost.gold ?? 0;
    totals.crystal += cost.crystal ?? 0;
  }
  return totals;
}
