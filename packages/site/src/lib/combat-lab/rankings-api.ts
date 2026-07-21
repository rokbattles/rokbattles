export const combatLabRankingSorts = [
  "overall",
  "damage",
  "rage",
  "assist",
  "sustainability",
  "trade",
  "consistency",
] as const;

export const combatLabRankingDirections = ["asc", "desc"] as const;

export type CombatLabRankingSort = (typeof combatLabRankingSorts)[number];
export type CombatLabRankingDirection = (typeof combatLabRankingDirections)[number];

export type CombatLabRanking = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  drastc: {
    overall: number;
    confidence: {
      score: number;
      uniqueGovernors: number;
      effectiveGovernors: number;
    };
    breakdown: Record<Exclude<CombatLabRankingSort, "overall">, number>;
  };
};

type CombatLabRankingsResponse = {
  items: CombatLabRanking[];
  refreshedAt: string | null;
};

export type CombatLabRankingsResult =
  | { status: "ready"; items: CombatLabRanking[]; refreshedAt: string | null }
  | { status: "error"; error: string };

const rankingsResultCache = new Map<string, Promise<CombatLabRankingsResult>>();

async function fetchCombatLabRankings(options: {
  sort: CombatLabRankingSort;
  direction: CombatLabRankingDirection;
}): Promise<CombatLabRankingsResponse> {
  const params = new URLSearchParams({
    sort: options.sort,
    direction: options.direction,
  });
  const response = await fetch(`/proxy/v1/global/combat-lab/rankings?${params}`, {
    cache: "no-store",
  });

  if (!response.ok) {
    throw new Error(`Failed to load Combat Lab rankings: ${response.status}`);
  }

  return (await response.json()) as CombatLabRankingsResponse;
}

export function loadCombatLabRankingsResult(options: {
  sort: CombatLabRankingSort;
  direction: CombatLabRankingDirection;
}): Promise<CombatLabRankingsResult> {
  const cacheKey = `${options.sort}:${options.direction}`;
  const cached = rankingsResultCache.get(cacheKey);

  if (cached) {
    return cached;
  }

  const resultPromise = fetchCombatLabRankings(options)
    .then(
      ({ items, refreshedAt }): CombatLabRankingsResult => ({
        status: "ready",
        items,
        refreshedAt,
      })
    )
    .catch((error: unknown): CombatLabRankingsResult => {
      rankingsResultCache.delete(cacheKey);

      return {
        status: "error",
        error: error instanceof Error ? error.message : "Failed to load Combat Lab rankings.",
      };
    });

  rankingsResultCache.set(cacheKey, resultPromise);
  return resultPromise;
}
