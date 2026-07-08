"use client";

import { useCallback, useEffect, useState } from "react";
import { toDateInput, todayUtcStartMillis } from "@/lib/loot/date";
import type { PersonalLootQueryResult } from "@/lib/types/loot";

export type PersonalLootEndpoint = "barbarians" | "barbarian-forts" | "baulurs" | "kahars-treasure";

type PersonalLootOptions = {
  governorId: number | null | undefined;
  endpoint: PersonalLootEndpoint;
  type?: string;
  levels?: number[];
  startParam?: string | null;
  endParam?: string | null;
};

export type UsePersonalLootResult = {
  data: PersonalLootQueryResult | null;
  loading: boolean;
  error: string | null;
};

export function defaultLootDateRange() {
  const currentYear = new Date().getUTCFullYear();
  return {
    start: `${currentYear}-01-01`,
    end: toDateInput(todayUtcStartMillis()),
  };
}

function buildParams(options: PersonalLootOptions) {
  const defaults = defaultLootDateRange();
  const params = new URLSearchParams({
    start: options.startParam || defaults.start,
    end: options.endParam || defaults.end,
  });

  if (options.type) {
    params.set("type", options.type);
  }

  if (options.levels?.length) {
    params.set("level", options.levels.join(","));
  }

  return params;
}

export function usePersonalLoot(options: PersonalLootOptions): UsePersonalLootResult {
  const { endpoint, governorId, type, levels, startParam, endParam } = options;
  const [data, setData] = useState<PersonalLootQueryResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchLoot = useCallback(async () => {
    if (governorId == null || !Number.isFinite(governorId)) {
      setData(null);
      setError(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const params = buildParams({ endpoint, governorId, type, levels, startParam, endParam });
      const response = await fetch(
        `/proxy/v1/governor/${governorId}/loot/${endpoint}?${params.toString()}`,
        { cache: "no-store" }
      );

      if (!response.ok) {
        throw new Error(`Failed to load loot: ${response.status}`);
      }

      setData((await response.json()) as PersonalLootQueryResult);
      setError(null);
    } catch (fetchError) {
      setData(null);
      setError(fetchError instanceof Error ? fetchError.message : "Failed to load loot.");
    } finally {
      setLoading(false);
    }
  }, [endpoint, endParam, governorId, levels, startParam, type]);

  useEffect(() => {
    void fetchLoot();
  }, [fetchLoot]);

  return { data, loading, error };
}
