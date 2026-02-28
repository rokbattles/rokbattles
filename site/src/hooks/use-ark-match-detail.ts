"use client";

import { useExtracted } from "next-intl";
import { useCallback, useEffect, useState } from "react";
import type { ArkMatchDetailResponse } from "@/lib/types/ark";

type DetailOptions = {
  governorId: number | null | undefined;
  matchId: string | null | undefined;
};

export type UseArkMatchDetailResult = {
  data: ArkMatchDetailResponse | null;
  loading: boolean;
  error: string | null;
};

export function useArkMatchDetail(options: DetailOptions): UseArkMatchDetailResult {
  const { governorId, matchId } = options;
  const t = useExtracted();
  const [data, setData] = useState<ArkMatchDetailResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchDetail = useCallback(async () => {
    if (governorId == null || !Number.isFinite(governorId) || !matchId) {
      setData(null);
      setError(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const response = await fetch(
        `/proxy/v1/governor/${governorId}/ark/${encodeURIComponent(matchId)}`,
        {
          cache: "no-store",
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to load Ark match details: ${response.status}`);
      }

      const payload = (await response.json()) as ArkMatchDetailResponse;
      setData(payload);
      setError(null);
    } catch (fetchError) {
      setData(null);
      setError(fetchError instanceof Error ? fetchError.message : t("Failed to fetch data"));
    } finally {
      setLoading(false);
    }
  }, [governorId, matchId, t]);

  useEffect(() => {
    void fetchDetail();
  }, [fetchDetail]);

  return {
    data,
    loading,
    error,
  };
}
