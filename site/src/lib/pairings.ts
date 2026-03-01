import type { EquipmentToken } from "@/lib/report/parsers";

export const PAIRINGS_GENERIC_ERROR = "Failed to load pairings.";

export function buildPairingsRangeParams(options: { startDate?: string; endDate?: string }) {
  const { startDate, endDate } = options;
  if (startDate && endDate) {
    return new URLSearchParams({ start: startDate, end: endDate });
  }

  const currentYear = new Date().getUTCFullYear();
  return new URLSearchParams({
    start: `${currentYear}-01-01`,
    end: `${currentYear}-12-31`,
  });
}

export type PairingTotals = {
  killScore: number;
  deaths: number;
  severelyWounded: number;
  wounded: number;
  enemyKillScore: number;
  enemyDeaths: number;
  enemySeverelyWounded: number;
  enemyWounded: number;
  dps: number;
  sps: number;
  tps: number;
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
