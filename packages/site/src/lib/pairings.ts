import type { EquipmentToken } from "@/lib/report/parsers";

export type PairingsActivity = "ark" | "home" | "kvk" | "strife";
export type PairingsBattleType = "open-field" | "swarming" | "rally" | "garrison";

export function buildPairingsRangeParams(options: {
  startDate: string;
  endDate: string;
  excludeActivities?: PairingsActivity[];
  excludeBattles?: PairingsBattleType[];
}) {
  const { startDate, endDate, excludeActivities, excludeBattles } = options;
  const params = new URLSearchParams({ start: startDate, end: endDate });

  if (excludeActivities && excludeActivities.length > 0) {
    params.set("excludeActivities", excludeActivities.join(","));
  }

  if (excludeBattles && excludeBattles.length > 0) {
    params.set("excludeBattles", excludeBattles.join(","));
  }

  return params;
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
  healingCount: number;
  enemyHealingCount: number;
  weightedTradePercent: number;
  battleDuration: number;
};

export type PairingAggregate = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  positiveTrades: number;
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

export type LoadoutArmament = {
  id: number;
  value: number;
};

export type LoadoutSnapshot = {
  equipment: EquipmentToken[];
  armaments: LoadoutArmament[];
  inscriptions: number[];
  formation: number | null;
};

export type PairingStats = {
  battles: number;
  kpGain: number;
  kpLoss: number;
  netKp: number;
  tradePercent: number | null;
};

export type PairingDetailLoadout = {
  key: string;
  stats: PairingStats;
  loadout: LoadoutSnapshot;
};

export type PairingDetailResponse = {
  range: PairingsRange;
  pairing: PairingStats | null;
  loadouts: PairingDetailLoadout[];
};

export type PairingMatchup = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  stats: PairingStats;
};

export type PairingMatchupsResponse = {
  range: PairingsRange;
  items: PairingMatchup[];
};

export type PairingDurationDistribution = {
  count: number;
  p10Millis: number;
  p25Millis: number;
  p50Millis: number;
  p75Millis: number;
  p90Millis: number;
};

export type PairingPerformanceMetrics = {
  battles: number;
  battleDurationMillis: number;
  durationDistribution: PairingDurationDistribution | null;
  kpGain: number;
  kpLoss: number;
  sevsGain: number;
  sevsLoss: number;
  powerLoss: number;
  opponentPowerLoss: number;
  tradePercent: number | null;
  dps: number | null;
  sps: number | null;
  tps: number | null;
  hps: number | null;
};

export type PairingPerformanceResponse = {
  range: PairingsRange;
  totals: PairingPerformanceMetrics;
  series: { date: string; metrics: PairingPerformanceMetrics }[];
};
