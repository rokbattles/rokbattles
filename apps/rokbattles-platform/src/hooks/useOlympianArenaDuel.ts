"use client";

import { useEffect, useState } from "react";

export type DuelReportEntry = {
  report: Record<string, unknown>;
};

export type DuelReportResponse = {
  duelId: number;
  items: DuelReportEntry[];
  count: number;
};

export function useOlympianArenaDuel(duelId: number | null | undefined) {
  const [data, setData] = useState<DuelReportResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (duelId == null) {
      setData(null);
      setError(null);
      setLoading(false);
      return;
    }

    let cancelled = false;

    setData(null);
    setLoading(true);
    setError(null);

    const fetchDuel = async () => {
      try {
        const res = await fetch(`/api/v2/olympian-arena/duel/${encodeURIComponent(duelId)}`);

        if (!res.ok) {
          throw new Error(`Failed to fetch duel: ${res.status}`);
        }

        const payload = (await res.json()) as DuelReportResponse;
        if (!cancelled) {
          setData(payload);
          setError(null);
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        if (!cancelled) {
          setError(message);
          setData(null);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    fetchDuel();

    return () => {
      cancelled = true;
    };
  }, [duelId]);

  return {
    data,
    loading,
    error,
  };
}
