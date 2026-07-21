import type { EquipmentToken } from "@/lib/report/parsers";

export const PAIRINGS_GENERIC_ERROR = "Failed to load pairings.";

export type PairingsActivity = "ark" | "home" | "kvk" | "strife";
export type PairingsBattleType = "open-field" | "swarming" | "rally" | "garrison";

export function buildPairingsRangeParams(options: {
  startDate?: string;
  endDate?: string;
  excludeActivities?: PairingsActivity[];
  excludeBattles?: PairingsBattleType[];
}) {
  const { startDate, endDate, excludeActivities, excludeBattles } = options;
  const params =
    startDate && endDate
      ? new URLSearchParams({ start: startDate, end: endDate })
      : new URLSearchParams({
          start: `${new Date().getUTCFullYear()}-01-01`,
          end: `${new Date().getUTCFullYear()}-12-31`,
        });

  if (excludeActivities && excludeActivities.length > 0) {
    params.set("excludeActivities", excludeActivities.join(","));
  }

  if (excludeBattles && excludeBattles.length > 0) {
    params.set("excludeBattles", excludeBattles.join(","));
  }

  return params;
}

export function formatExcludedPairingsFilters<T extends string>(
  excluded: T[],
  options: Record<T, string>
) {
  if (excluded.length === 0) {
    return null;
  }

  return excluded.map((value) => options[value]).join(", ");
}

export type PairingTotals = {
  killScore: number;
  deaths: number;
  severelyWounded: number;
  wounded: number;
  powerLoss: number;
  atkPowerLoss: number;
  skillPowerLoss: number;
  enemyKillScore: number;
  enemyDeaths: number;
  enemySeverelyWounded: number;
  enemyWounded: number;
  enemyPowerLoss: number;
  enemyAtkPowerLoss: number;
  enemySkillPowerLoss: number;
  dps: number;
  sps: number;
  tps: number;
  hps: number;
  tradePercent: number;
  weightedTradePercent: number;
  battleDuration: number;
};

export type PairingAggregate = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: PairingTotals;
};

export type PairingsRange = {
  start: string;
  end: string;
};

export type PairingsResponse = {
  range: PairingsRange;
  items: PairingAggregate[];
};

export type PairingsResult = {
  data: PairingAggregate[];
  loading: boolean;
  error: string | null;
  range: PairingsRange | null;
};

export type LoadoutGranularity = "exact" | "simplified";

export type LoadoutArmament = {
  id: number;
  value?: number;
};

export type LoadoutSnapshot = {
  equipment: EquipmentToken[];
  armaments: LoadoutArmament[];
  inscriptions: number[];
  formation: number | null;
};

export type LoadoutAggregate = {
  key: string;
  count: number;
  totals: PairingTotals;
  loadout: LoadoutSnapshot;
};

export type PairingLoadoutsResponse = {
  range: PairingsRange;
  items: LoadoutAggregate[];
};

export type PairingLoadoutsResult = {
  data: LoadoutAggregate[];
  loading: boolean;
  error: string | null;
};

export type OpponentGranularity = "overall" | LoadoutGranularity;

export type OpponentAggregate = {
  enemyPrimaryCommanderId: number;
  enemySecondaryCommanderId: number;
  count: number;
  totals: PairingTotals;
};

export type PairingOpponentsResponse = {
  range: PairingsRange;
  items: OpponentAggregate[];
};

export type PairingOpponentsResult = {
  data: OpponentAggregate[];
  loading: boolean;
  error: string | null;
};
