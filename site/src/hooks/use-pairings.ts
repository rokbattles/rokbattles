"use client";

import { useCallback, useEffect, useState } from "react";
import type {
  PairingAggregate,
  PairingsRange,
  PairingsResponse,
  PairingsResult,
} from "@/lib/pairings";
import { buildPairingsRangeParams, PAIRINGS_GENERIC_ERROR } from "@/lib/pairings";

export type {
  LoadoutAggregate,
  LoadoutGranularity,
  LoadoutSnapshot,
  OpponentGranularity,
} from "@/lib/pairings";

type PairingsOptions = {
  governorId: number | null | undefined;
  startDate?: string;
  endDate?: string;
};

export function usePairings(options: PairingsOptions): PairingsResult {
  const { governorId, startDate, endDate } = options;
  const [pairings, setPairings] = useState<PairingAggregate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [range, setRange] = useState<PairingsRange | null>(null);

  const fetchPairings = useCallback(async () => {
    if (governorId == null) {
      setPairings([]);
      setRange(null);
      setError(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const params = buildPairingsRangeParams({ startDate, endDate });
      const res = await fetch(`/proxy/v1/governor/${governorId}/pairings?${params}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(`Failed to load pairings: ${res.status}`);
      }

      const data = (await res.json()) as PairingsResponse;
      setPairings(Array.isArray(data.items) ? data.items : []);
      setRange(data.range ?? null);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : PAIRINGS_GENERIC_ERROR;
      setPairings([]);
      setRange(null);
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
    range,
  };
}
