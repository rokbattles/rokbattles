export type LootExplorerResponse<T> = {
  items: T[];
};

export type LootQuantityRange = {
  min: number;
  max: number;
};

export type LootRange = {
  min: number | null;
  max: number | null;
};

export type LootDrop = {
  type: number;
  subType: number;
  results: number;
  dropRate: number;
  quantity: LootQuantityRange;
  totalQuantity: number;
  averageQuantity: number;
};

export type BarbarianLootDocument = {
  kind: number;
  level: number;
  loot: LootDrop[];
  data: {
    bType: number;
    apCost: number;
    honorPoints: number;
    baseXp: number;
  };
  totals: {
    results: number;
    apUsed: number;
    honorPointsGained: number;
    xpGained: number;
  };
  refreshedAt: string;
};

export type BarbarianFortLootDocument = {
  kind: number;
  level: number;
  rewardTiers: Array<{
    tier: number;
    results: number;
    receiveRate: number;
    damagePercentage: LootRange;
    loot: LootDrop[];
  }>;
  data: {
    apCost: number;
    honorPoints: number;
  };
  totals: {
    results: number;
    apUsed: number;
    honorPointsGained: number;
  };
  refreshedAt: string;
};

export type BaulurRollSlot = {
  slot: number;
  results: number;
  noItemResults: number;
  noItemRate: number;
  loot: LootDrop[];
};

export type BaulurLootDocument = {
  kind: number;
  resourcePool: {
    results: number;
    damageFactor: LootRange;
    loot: LootDrop[];
  };
  rollPool: {
    results: number;
    matchedResults: number;
    unmatchedResults: number;
    damageFactor: LootRange;
    slots: BaulurRollSlot[];
  };
  totals: {
    results: number;
    reports: number;
    duplicateReports: number;
  };
  refreshedAt: string;
};

export type KaharTreasureLootDocument = {
  kind: string;
  loot: LootDrop[];
  totals: {
    results: number;
    apUsed: number;
  };
  refreshedAt: string;
};

export type KaruakCeremonyLootDocument = {
  kind: number;
  slots: Array<{
    slot: number;
    results: number;
    loot: LootDrop[];
  }>;
  totals: {
    results: number;
    reports: number;
    duplicateReports: number;
    emptyResults: number;
    unmatchedResults: number;
  };
  refreshedAt: string;
};

type LootExplorerEndpoint = "barbarians" | "barbarian-forts" | "baulurs" | "karuak-ceremony";

export async function fetchLootExplorerItems<T>(
  endpoint: LootExplorerEndpoint
): Promise<LootExplorerResponse<T>> {
  const response = await fetch(`/proxy/v1/global/loot-explorer/${endpoint}`, {
    cache: "no-store",
  });

  if (!response.ok) {
    throw new Error(`Failed to load loot explorer data: ${response.status}`);
  }

  return (await response.json()) as LootExplorerResponse<T>;
}

export async function fetchKaharTreasureLoot(): Promise<KaharTreasureLootDocument> {
  const response = await fetch("/proxy/v1/global/loot-explorer/kahars-treasure", {
    cache: "no-store",
  });

  if (!response.ok) {
    throw new Error(`Failed to load Kahar treasure loot data: ${response.status}`);
  }

  return (await response.json()) as KaharTreasureLootDocument;
}
