"use client";

import { useCallback, useEffect, useState } from "react";
import type {
  OpponentAggregate,
  OpponentGranularity,
  PairingOpponentsResponse,
  PairingOpponentsResult,
  PairingsReportType,
} from "@/lib/pairings";
import { buildPairingsRangeParams, PAIRINGS_GENERIC_ERROR } from "@/lib/pairings";

export function usePairingOpponents(options: {
  governorId: number | null | undefined;
  primaryCommanderId: number | null;
  secondaryCommanderId: number | null;
  granularity: OpponentGranularity;
  loadoutKey?: string | null;
  startDate?: string;
  endDate?: string;
  excludeTypes?: PairingsReportType[];
}): PairingOpponentsResult {
  const {
    governorId,
    primaryCommanderId,
    secondaryCommanderId,
    granularity,
    loadoutKey,
    startDate,
    endDate,
    excludeTypes,
  } = options;
  const [data, setData] = useState<OpponentAggregate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchOpponents = useCallback(async () => {
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

    const params = buildPairingsRangeParams({ startDate, endDate, excludeTypes });
    params.set("primary", String(primaryCommanderId));
    params.set("secondary", String(secondaryCommanderId));
    params.set("granularity", granularity);
    if (granularity !== "overall" && loadoutKey) {
      params.set("loadoutKey", loadoutKey);
    }

    try {
      const res = await fetch(`/proxy/v1/governor/${governorId}/pairings/opponents?${params}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(`Failed to load pairings: ${res.status}`);
      }

      const payload = (await res.json()) as PairingOpponentsResponse;
      setData(Array.isArray(payload.items) ? payload.items : []);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : PAIRINGS_GENERIC_ERROR;
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
    startDate,
    endDate,
    excludeTypes,
  ]);

  useEffect(() => {
    setData([]);
    setError(null);

    void fetchOpponents();
  }, [fetchOpponents]);

  return { data, loading, error };
}
