"use client";

import { useCallback, useEffect, useState } from "react";
import type { EquipmentToken } from "@/lib/report/parsers";

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

export type PairingsResponse = {
  year: number;
  items: PairingAggregate[];
};

export type PairingsResult = {
  data: PairingAggregate[];
  loading: boolean;
  error: string | null;
  year: number | null;
};

export type LoadoutGranularity = "exact" | "normalized";

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
  items: LoadoutAggregate[];
};

export type PairingLoadoutsResult = {
  data: LoadoutAggregate[];
  loading: boolean;
  error: string | null;
};

export type EnemyGranularity = "overall" | LoadoutGranularity;

export type EnemyAggregate = {
  enemyPrimaryCommanderId: number;
  enemySecondaryCommanderId: number;
  count: number;
  totals: PairingTotals;
};

export type PairingEnemiesResponse = {
  items: EnemyAggregate[];
};

export type PairingEnemiesResult = {
  data: EnemyAggregate[];
  loading: boolean;
  error: string | null;
};

const DEFAULT_YEAR = new Date().getUTCFullYear();
const GENERIC_ERROR = "Failed to load pairings.";

type PairingsOptions = {
  governorId: number | null | undefined;
  startDate?: string;
  endDate?: string;
};

function buildRangeParams(options: { startDate?: string; endDate?: string; year?: number | null }) {
  const { startDate, endDate, year } = options;
  if (startDate && endDate) {
    return new URLSearchParams({ start: startDate, end: endDate });
  }

  return new URLSearchParams({ year: String(year ?? DEFAULT_YEAR) });
}

export function usePairings(options: PairingsOptions): PairingsResult {
  const { governorId, startDate, endDate } = options;
  const [pairings, setPairings] = useState<PairingAggregate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [year, setYear] = useState<number | null>(null);

  const fetchPairings = useCallback(async () => {
    if (governorId == null) {
      setPairings([]);
      setYear(null);
      setError(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const params = buildRangeParams({ startDate, endDate });
      const res = await fetch(`/api/v2/governor/${governorId}/pairings?${params}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(`Failed to load pairings: ${res.status}`);
      }

      const data = (await res.json()) as PairingsResponse;
      setPairings(Array.isArray(data.items) ? data.items : []);
      setYear(data.year ?? null);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : GENERIC_ERROR;
      setPairings([]);
      setYear(null);
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [governorId, startDate, endDate]);

  useEffect(() => {
    setPairings([]);
    setError(null);

    void fetchPairings();
  }, [fetchPairings]);

  return {
    data: pairings,
    loading,
    error,
    year,
  };
}

export function usePairingLoadouts(options: {
  governorId: number | null | undefined;
  primaryCommanderId: number | null;
  secondaryCommanderId: number | null;
  granularity: LoadoutGranularity;
  year?: number | null;
  startDate?: string;
  endDate?: string;
}): PairingLoadoutsResult {
  const {
    governorId,
    primaryCommanderId,
    secondaryCommanderId,
    granularity,
    year,
    startDate,
    endDate,
  } = options;
  const [data, setData] = useState<LoadoutAggregate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchLoadouts = useCallback(async () => {
    if (
      governorId == null ||
      primaryCommanderId == null ||
      secondaryCommanderId == null ||
      !Number.isFinite(primaryCommanderId) ||
      !Number.isFinite(secondaryCommanderId)
    ) {
      setData([]);
      setError(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    const params = buildRangeParams({ startDate, endDate, year });
    params.set("primary", String(primaryCommanderId));
    params.set("secondary", String(secondaryCommanderId));
    params.set("granularity", granularity);

    try {
      const res = await fetch(`/api/v2/governor/${governorId}/pairings/loadouts?${params}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(`Failed to load pairings: ${res.status}`);
      }

      const payload = (await res.json()) as PairingLoadoutsResponse;
      setData(Array.isArray(payload.items) ? payload.items : []);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : GENERIC_ERROR;
      setData([]);
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [governorId, primaryCommanderId, secondaryCommanderId, granularity, year, startDate, endDate]);

  useEffect(() => {
    setData([]);
    setError(null);

    void fetchLoadouts();
  }, [fetchLoadouts]);

  return { data, loading, error };
}

export function usePairingEnemies(options: {
  governorId: number | null | undefined;
  primaryCommanderId: number | null;
  secondaryCommanderId: number | null;
  granularity: EnemyGranularity;
  loadoutKey?: string | null;
  year?: number | null;
  startDate?: string;
  endDate?: string;
}): PairingEnemiesResult {
  const {
    governorId,
    primaryCommanderId,
    secondaryCommanderId,
    granularity,
    loadoutKey,
    year,
    startDate,
    endDate,
  } = options;
  const [data, setData] = useState<EnemyAggregate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchEnemies = useCallback(async () => {
    if (
      governorId == null ||
      primaryCommanderId == null ||
      secondaryCommanderId == null ||
      !Number.isFinite(primaryCommanderId) ||
      !Number.isFinite(secondaryCommanderId)
    ) {
      setData([]);
      setError(null);
      setLoading(false);
      return;
    }

    if (granularity !== "overall" && !loadoutKey) {
      setData([]);
      setError(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    const params = buildRangeParams({ startDate, endDate, year });
    params.set("primary", String(primaryCommanderId));
    params.set("secondary", String(secondaryCommanderId));
    params.set("granularity", granularity);
    if (granularity !== "overall" && loadoutKey) {
      params.set("loadoutKey", loadoutKey);
    }

    try {
      const res = await fetch(`/api/v2/governor/${governorId}/pairings/enemies?${params}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(`Failed to load pairings: ${res.status}`);
      }

      const payload = (await res.json()) as PairingEnemiesResponse;
      setData(Array.isArray(payload.items) ? payload.items : []);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : GENERIC_ERROR;
      setData([]);
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [
    governorId,
    primaryCommanderId,
    secondaryCommanderId,
    granularity,
    loadoutKey,
    year,
    startDate,
    endDate,
  ]);

  useEffect(() => {
    setData([]);
    setError(null);

    void fetchEnemies();
  }, [fetchEnemies]);

  return { data, loading, error };
}
