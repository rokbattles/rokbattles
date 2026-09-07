import type {
  BuildingCost,
  BuildingCostSchedule,
  BuildingKind,
  PlannedBuilding,
  TerritoryApiBuildingConfig,
} from "./types";

export type CostTotals = Required<BuildingCost> & { unknown: number };

export type BuildingCostProgress = {
  built: number;
  limit: number;
  next: BuildingCost | null;
};

export type BuildingCostEntry = {
  building: PlannedBuilding;
  number: number;
  cost: BuildingCost | null;
};

function costAt(schedule: BuildingCostSchedule, kind: BuildingKind, number: number) {
  return schedule[kind]?.find((tier) => number >= tier.from && number <= tier.to)?.cost ?? null;
}

export function buildingCostProgress(
  kind: BuildingKind,
  buildings: PlannedBuilding[],
  schedule: BuildingCostSchedule,
  allianceId: string,
  buildingConfigs: Partial<Record<BuildingKind, TerritoryApiBuildingConfig>>
): BuildingCostProgress {
  const built = buildings.filter(
    (building) => building.allianceId === allianceId && building.kind === kind
  ).length;
  return {
    built,
    limit: buildingConfigs[kind]?.limit ?? 0,
    next: costAt(schedule, kind, built + 1),
  };
}

export function buildingCostBreakdown(
  buildings: PlannedBuilding[],
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
  buildings: PlannedBuilding[],
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
