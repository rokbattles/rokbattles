"use client";

import { useExtracted } from "next-intl";
import { useCallback, useEffect, useState } from "react";
import type { ArkMatchHistoryResult } from "@/lib/types/ark";

type HistoryOptions = {
  governorId: number | null | undefined;
  limit?: number;
};

export type UseArkMatchHistoryResult = {
  data: ArkMatchHistoryResult | null;
  loading: boolean;
  error: string | null;
};

function buildHistoryParams(limit?: number) {
  const params = new URLSearchParams();

  if (typeof limit === "number" && Number.isFinite(limit)) {
    params.set("limit", String(Math.max(1, Math.floor(limit))));
  }

  return params;
}

export function useArkMatchHistory(options: HistoryOptions): UseArkMatchHistoryResult {
  const { governorId, limit } = options;
  const t = useExtracted();
  const [data, setData] = useState<ArkMatchHistoryResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchHistory = useCallback(async () => {
    if (governorId == null || !Number.isFinite(governorId)) {
      setData(null);
      setError(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const params = buildHistoryParams(limit);
      const query = params.toString();
      const response = await fetch(
        `/proxy/v1/governor/${governorId}/ark${query ? `?${query}` : ""}`,
        {
          cache: "no-store",
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to load Ark match history: ${response.status}`);
      }

      const payload = (await response.json()) as ArkMatchHistoryResult;
      setData(payload);
      setError(null);
    } catch (fetchError) {
      setData(null);
      setError(fetchError instanceof Error ? fetchError.message : t("Failed to fetch data"));
    } finally {
      setLoading(false);
    }
  }, [governorId, limit, t]);

  useEffect(() => {
    void fetchHistory();
  }, [fetchHistory]);

  return {
    data,
    loading,
    error,
  };
}
