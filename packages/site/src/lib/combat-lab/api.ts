export type CombatLabCategoryScore = {
  value: number;
  p10: number;
  p90: number;
  score: number;
};

export type CombatLabDrastcConfidence = {
  score: number;
  uniqueGovernors: number;
  effectiveGovernors: number;
};

export type CombatLabDrastcScore = {
  samples: number;
  breakdown: {
    damage: CombatLabCategoryScore;
    rage: CombatLabCategoryScore;
    assist: CombatLabCategoryScore;
    sustainability: CombatLabCategoryScore;
    trade: CombatLabCategoryScore;
    consistency: CombatLabCategoryScore;
  };
  overall: number;
  confidence: CombatLabDrastcConfidence;
};

export type CombatLabSummary = {
  totalBattles: number;
  killPointsGained: number;
  killPointsLost: number;
  avgTradePercentage: number;
  weightedTradePercentage: number;
  avgBattleDuration: number;
  totalBattleDuration: number;
  severelyWoundedInflicted: number;
  severelyWoundedTaken: number;
  dps: number;
  sps: number;
  tps: number;
  hps: number;
};

export type CombatLabFormation = {
  id: number;
  count: number;
};

export type CombatLabStrategySummary = CombatLabSummary & {
  formations: CombatLabFormation[];
};

export type CombatLabStrategies = {
  all: CombatLabStrategySummary;
  openField: CombatLabStrategySummary;
  swarming: CombatLabStrategySummary;
  rally: CombatLabStrategySummary;
  garrison: CombatLabStrategySummary;
};

export type CombatLabPairingDocument = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  summary: CombatLabSummary;
  strategies: CombatLabStrategies;
  drastc: CombatLabDrastcScore | null;
  refreshedAt: string;
};

export type CombatLabPairingResult =
  | { status: "ready"; item: CombatLabPairingDocument }
  | { status: "error"; error: string };

const pairingResultCache = new Map<string, Promise<CombatLabPairingResult>>();

async function fetchCombatLabPairing(options: {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  signal?: AbortSignal;
}): Promise<CombatLabPairingDocument> {
  const params = new URLSearchParams({
    primaryCommanderId: options.primaryCommanderId.toString(),
    secondaryCommanderId: options.secondaryCommanderId.toString(),
  });

  const response = await fetch(`/proxy/v1/global/combat-lab?${params}`, {
    cache: "no-store",
    signal: options.signal,
  });

  if (!response.ok) {
    throw new Error(`Failed to load Combat Lab pairing: ${response.status}`);
  }

  return (await response.json()) as CombatLabPairingDocument;
}

export function loadCombatLabPairingResult(options: {
  primaryCommanderId: number;
  secondaryCommanderId: number;
}): Promise<CombatLabPairingResult> {
  const cacheKey = `${options.primaryCommanderId}:${options.secondaryCommanderId}`;
  const cached = pairingResultCache.get(cacheKey);

  if (cached) {
    return cached;
  }

  const resultPromise = fetchCombatLabPairing(options)
    .then((item): CombatLabPairingResult => ({ status: "ready", item }))
    .catch((error: unknown): CombatLabPairingResult => {
      pairingResultCache.delete(cacheKey);

      return {
        status: "error",
        error: error instanceof Error ? error.message : "Failed to load Combat Lab pairing.",
      };
    });

  pairingResultCache.set(cacheKey, resultPromise);
  return resultPromise;
}
