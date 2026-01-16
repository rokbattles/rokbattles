"use client";

import { useCallback, useEffect, useState } from "react";

export type RewardAggregate = {
  type: number;
  subType: number;
  total: number;
  count: number;
};

export type RewardStats = {
  totalReports: number;
  barbKills: number;
  barbFortKills: number;
  otherNpcKills: number;
};

export type RewardsResponse = {
  year: number;
  stats: RewardStats;
  rewards: RewardAggregate[];
};

export type RewardsResult = {
  data: RewardsResponse | null;
  loading: boolean;
  error: string | null;
  year: number | null;
};

const DEFAULT_YEAR = new Date().getUTCFullYear();
const GENERIC_ERROR = "Failed to load rewards.";

type RewardsOptions = {
  governorId: number | null | undefined;
  startDate?: string;
  endDate?: string;
};

function buildRangeParams(options: {
  startDate?: string;
  endDate?: string;
  year?: number | null;
}) {
  const { startDate, endDate, year } = options;
  if (startDate && endDate) {
    return new URLSearchParams({ start: startDate, end: endDate });
  }

  return new URLSearchParams({ year: String(year ?? DEFAULT_YEAR) });
}

export function useRewards(options: RewardsOptions): RewardsResult {
  const { governorId, startDate, endDate } = options;
  const [data, setData] = useState<RewardsResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [year, setYear] = useState<number | null>(null);

  const fetchRewards = useCallback(async () => {
    if (governorId == null) {
      setData(null);
      setYear(null);
      setError(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const params = buildRangeParams({ startDate, endDate });
      const res = await fetch(
        `/api/v2/governor/${governorId}/rewards?${params}`,
        {
          cache: "no-store",
        }
      );

      if (!res.ok) {
        throw new Error(`Failed to load rewards: ${res.status}`);
      }

      const payload = (await res.json()) as RewardsResponse;
      setData(payload ?? null);
      setYear(payload?.year ?? null);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : GENERIC_ERROR;
      setData(null);
      setYear(null);
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [governorId, startDate, endDate]);

  useEffect(() => {
    setData(null);
    setError(null);

    void fetchRewards();
  }, [fetchRewards]);

  return { data, loading, error, year };
}
