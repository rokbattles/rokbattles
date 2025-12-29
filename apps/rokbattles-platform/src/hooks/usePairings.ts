"use client";

import { useCallback, useContext, useEffect, useState } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";

export type GovernorMarchTotals = {
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

export type GovernorMarchAggregate = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: GovernorMarchTotals;
  averageKillScore: number;
  previousTotals: GovernorMarchTotals;
  previousCount: number;
  monthly: GovernorMonthlyAggregate[];
};

export type GovernorMonthlyAggregate = {
  monthKey: string;
  count: number;
  totals: GovernorMarchTotals;
};

type GovernorMarchesResponse = {
  year: number;
  governorId: number;
  period: {
    start: string;
    end: string;
  };
  count: number;
  items: GovernorMarchAggregate[];
};

export type UsePairingsResult = {
  data: GovernorMarchAggregate[];
  loading: boolean;
  error: string | null;
  period: GovernorMarchesResponse["period"] | null;
  year: number | null;
  reload: () => Promise<void>;
};

const TARGET_YEAR = 2025;

export function usePairings(): UsePairingsResult {
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("usePairings must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;
  const governorId = activeGovernor?.governorId;

  const [pairings, setPairings] = useState<GovernorMarchAggregate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [period, setPeriod] = useState<GovernorMarchesResponse["period"] | null>(null);
  const [year, setYear] = useState<number | null>(null);

  const fetchPairings = useCallback(async () => {
    if (governorId == null) {
      setPairings([]);
      setPeriod(null);
      setYear(null);
      setError(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const res = await fetch(`/api/v2/governor/${governorId}/marches?year=${TARGET_YEAR}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(`Failed to load marches: ${res.status}`);
      }

      const data = (await res.json()) as GovernorMarchesResponse;
      setPairings(Array.isArray(data.items) ? data.items : []);
      setPeriod(data.period ?? null);
      setYear(data.year ?? null);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setPairings([]);
      setPeriod(null);
      setYear(null);
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [governorId]);

  useEffect(() => {
    setPairings([]);
    setPeriod(null);
    setError(null);

    void fetchPairings();
  }, [fetchPairings]);

  const reload = useCallback(async () => {
    await fetchPairings();
  }, [fetchPairings]);

  return {
    data: pairings,
    loading,
    error,
    period,
    year,
    reload,
  };
}
