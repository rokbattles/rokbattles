"use client";

import { useCallback, useEffect, useState } from "react";
import type {
  LoadoutAggregate,
  LoadoutGranularity,
  PairingLoadoutsResponse,
  PairingLoadoutsResult,
} from "@/lib/pairings";
import { buildPairingsRangeParams, PAIRINGS_GENERIC_ERROR } from "@/lib/pairings";

export function usePairingLoadouts(options: {
  governorId: number | null | undefined;
  primaryCommanderId: number | null;
  secondaryCommanderId: number | null;
  granularity: LoadoutGranularity;
  startDate?: string;
  endDate?: string;
}): PairingLoadoutsResult {
  const { governorId, primaryCommanderId, secondaryCommanderId, granularity, startDate, endDate } =
    options;
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

    const params = buildPairingsRangeParams({ startDate, endDate });
    params.set("primary", String(primaryCommanderId));
    params.set("secondary", String(secondaryCommanderId));
    params.set("granularity", granularity);

    try {
      const res = await fetch(`/proxy/v1/governor/${governorId}/pairings/loadouts?${params}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(`Failed to load pairings: ${res.status}`);
      }

      const payload = (await res.json()) as PairingLoadoutsResponse;
      setData(Array.isArray(payload.items) ? payload.items : []);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : PAIRINGS_GENERIC_ERROR;
      setData([]);
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [governorId, primaryCommanderId, secondaryCommanderId, granularity, startDate, endDate]);

  useEffect(() => {
    setData([]);
    setError(null);

    void fetchLoadouts();
  }, [fetchLoadouts]);

  return { data, loading, error };
}
