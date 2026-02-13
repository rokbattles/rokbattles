"use client";

import { useCallback, useEffect, useState } from "react";
import type { FavoriteReportType } from "@/lib/types/favorite";

type UseReportFavoriteOptions = {
  parentHash?: string | null;
  reportType?: FavoriteReportType;
  enabled?: boolean;
};

type FavoriteStatusResponse = {
  favorited: boolean;
};

export function useReportFavorite({
  parentHash,
  reportType = "battle",
  enabled = true,
}: UseReportFavoriteOptions) {
  const [favorited, setFavorited] = useState(false);
  const [loading, setLoading] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!enabled || !parentHash) {
      setFavorited(false);
      setLoading(false);
      setError(null);
      return;
    }

    let cancelled = false;

    setLoading(true);
    setError(null);

    const fetchStatus = async () => {
      try {
        const response = await fetch(
          `/api/v2/favorites/${encodeURIComponent(parentHash)}?reportType=${reportType}`,
          { cache: "no-store" }
        );

        if (!response.ok) {
          throw new Error(`Failed to fetch favorite status: ${response.status}`);
        }

        const payload = (await response.json()) as FavoriteStatusResponse;
        if (!cancelled) {
          setFavorited(Boolean(payload?.favorited));
        }
      } catch (err) {
        if (!cancelled) {
          const message = err instanceof Error ? err.message : String(err);
          setError(message);
          setFavorited(false);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    fetchStatus();

    return () => {
      cancelled = true;
    };
  }, [enabled, parentHash, reportType]);

  const toggleFavorite = useCallback(async () => {
    if (!enabled || !parentHash || updating) {
      return;
    }

    setUpdating(true);
    setError(null);

    try {
      const response = await fetch(
        `/api/v2/favorites/${encodeURIComponent(parentHash)}?reportType=${reportType}`,
        {
          method: favorited ? "DELETE" : "POST",
          cache: "no-store",
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to update favorite: ${response.status}`);
      }

      const payload = (await response.json()) as FavoriteStatusResponse;
      setFavorited(Boolean(payload?.favorited));
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setUpdating(false);
    }
  }, [enabled, favorited, parentHash, reportType, updating]);

  return {
    favorited,
    loading,
    updating,
    error,
    toggleFavorite,
  };
}
