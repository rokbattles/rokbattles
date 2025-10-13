"use client";

import { useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
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
};

export type GovernorMarchAggregate = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: GovernorMarchTotals;
  averageKillScore: number;
  previousTotals: GovernorMarchTotals;
  previousCount: number;
};

type GovernorMarchesResponse = {
  governorId: number;
  period: {
    start: string;
    end: string;
  };
  count: number;
  items: GovernorMarchAggregate[];
};

export type UseMyPairingsResult = {
  data: GovernorMarchAggregate[];
  loading: boolean;
  error: string | null;
  period: GovernorMarchesResponse["period"] | null;
  reload: () => Promise<void>;
};

export function useMyPairings(): UseMyPairingsResult {
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("useMyPairings must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;
  const governorId = activeGovernor?.governorId;

  const [pairings, setPairings] = useState<GovernorMarchAggregate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [period, setPeriod] = useState<GovernorMarchesResponse["period"] | null>(null);

  const requestIdRef = useRef(0);
  const isMountedRef = useRef(true);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  const fetchPairings = useCallback(async () => {
    if (governorId == null) {
      if (isMountedRef.current) {
        setPairings([]);
        setPeriod(null);
        setError(null);
        setLoading(false);
      }
      return;
    }

    const requestId = ++requestIdRef.current;
    setLoading(true);
    setError(null);

    try {
      const res = await fetch(`/api/v2/governor/${governorId}/marches`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(`Failed to load marches: ${res.status}`);
      }

      const data = (await res.json()) as GovernorMarchesResponse;
      const isLatest = requestIdRef.current === requestId;
      if (!isMountedRef.current || !isLatest) {
        return;
      }

      setPairings(Array.isArray(data.items) ? data.items : []);
      setPeriod(data.period ?? null);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      const isLatest = requestIdRef.current === requestId;
      if (!isMountedRef.current || !isLatest) {
        return;
      }
      setPairings([]);
      setPeriod(null);
      setError(message);
    } finally {
      const isLatest = requestIdRef.current === requestId;
      if (isLatest && isMountedRef.current) {
        setLoading(false);
      }
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

  return useMemo(
    () => ({
      data: pairings,
      loading,
      error,
      period,
      reload,
    }),
    [pairings, loading, error, period, reload]
  );
}
