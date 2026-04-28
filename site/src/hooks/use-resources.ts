"use client";

import { useCallback, useEffect, useState } from "react";
import { toDateInput, todayUtcStartMillis } from "@/lib/loot/date";
import type { ResourcesQueryResult } from "@/lib/types/resources";

type ResourcesOptions = {
  governorId: number | null | undefined;
  startParam?: string | null;
  endParam?: string | null;
};

export type UseResourcesResult = {
  data: ResourcesQueryResult | null;
  loading: boolean;
  error: string | null;
};

function buildRangeParams(options: { startParam?: string | null; endParam?: string | null }) {
  const { startParam, endParam } = options;
  if (startParam && endParam) {
    return new URLSearchParams({ start: startParam, end: endParam });
  }

  const currentYear = new Date().getUTCFullYear();
  return new URLSearchParams({
    start: `${currentYear}-01-01`,
    end: toDateInput(todayUtcStartMillis()),
  });
}

export function useResources(options: ResourcesOptions): UseResourcesResult {
  const { governorId, startParam, endParam } = options;
  const [data, setData] = useState<ResourcesQueryResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchResources = useCallback(async () => {
    if (governorId == null || !Number.isFinite(governorId)) {
      setData(null);
      setError(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const params = buildRangeParams({ startParam, endParam });
      const response = await fetch(
        `/proxy/v1/governor/${governorId}/resources?${params.toString()}`,
        {
          cache: "no-store",
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to load resources: ${response.status}`);
      }

      const payload = (await response.json()) as ResourcesQueryResult;
      setData(payload);
      setError(null);
    } catch (fetchError) {
      setData(null);
      setError(fetchError instanceof Error ? fetchError.message : "Failed to load resources.");
    } finally {
      setLoading(false);
    }
  }, [endParam, governorId, startParam]);

  useEffect(() => {
    void fetchResources();
  }, [fetchResources]);

  return { data, loading, error };
}
