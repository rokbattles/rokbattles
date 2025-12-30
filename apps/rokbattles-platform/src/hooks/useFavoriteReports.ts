"use client";

import { useCallback, useEffect, useState } from "react";
import type { ReportSummary } from "@/hooks/useReports";
import type { FavoriteReportType } from "@/lib/types/favorite";

type FavoritesApiResponse = {
  items: ReportSummary[];
  count: number;
  cursor?: string;
};

export type UseFavoriteReportsResult = {
  data: ReportSummary[];
  loading: boolean;
  error: string | null;
  cursor: string | undefined;
  loadMore: () => Promise<void>;
};

type UseFavoriteReportsOptions = {
  reportType?: FavoriteReportType;
};

export function useFavoriteReports({
  reportType = "battle",
}: UseFavoriteReportsOptions = {}): UseFavoriteReportsResult {
  const [favorites, setFavorites] = useState<ReportSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [cursor, setCursor] = useState<string | undefined>(undefined);

  const fetchFavorites = useCallback(
    async (nextCursor?: string) => {
      const params = new URLSearchParams();
      params.set("reportType", reportType);
      if (nextCursor) {
        params.set("cursor", nextCursor);
      }

      const res = await fetch(`/api/v2/favorites?${params.toString()}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(`Failed to fetch favorites: ${res.status}`);
      }

      return (await res.json()) as FavoritesApiResponse;
    },
    [reportType]
  );

  useEffect(() => {
    let cancelled = false;

    setCursor(undefined);
    setFavorites([]);
    setError(null);
    setLoading(true);

    fetchFavorites()
      .then((data) => {
        if (cancelled) {
          return;
        }
        setCursor(data.cursor ?? undefined);
        setFavorites(data.items);
        setError(null);
      })
      .catch((err) => {
        if (cancelled) {
          return;
        }
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [fetchFavorites]);

  const loadMore = async () => {
    if (loading) {
      return;
    }

    if (!cursor) {
      return;
    }

    setLoading(true);

    try {
      const data = await fetchFavorites(cursor);
      setCursor(data.cursor ?? undefined);
      setFavorites((prev) => [...prev, ...data.items]);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  return {
    data: favorites,
    loading,
    error,
    cursor,
    loadMore,
  };
}
