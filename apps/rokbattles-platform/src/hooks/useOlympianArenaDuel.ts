"use client";

import { useEffect, useMemo, useRef, useState } from "react";

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

  const requestIdRef = useRef(0);
  const isMountedRef = useRef(true);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (duelId == null) {
      setData(null);
      setError(null);
      setLoading(false);
      return;
    }

    const requestId = ++requestIdRef.current;
    const controller = new AbortController();

    setData(null);
    setLoading(true);
    setError(null);

    const fetchDuel = async () => {
      try {
        const res = await fetch(`/api/v2/olympian-arena/duel/${encodeURIComponent(duelId)}`, {
          signal: controller.signal,
        });

        if (!res.ok) {
          throw new Error(`Failed to fetch duel: ${res.status}`);
        }

        const payload = (await res.json()) as DuelReportResponse;

        const isLatest = requestIdRef.current === requestId;
        if (!isMountedRef.current || !isLatest) {
          return;
        }

        setData(payload);
        setError(null);
      } catch (err) {
        if (controller.signal.aborted) {
          return;
        }

        const isLatest = requestIdRef.current === requestId;
        if (!isMountedRef.current || !isLatest) {
          return;
        }

        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setData(null);
      } finally {
        const isLatest = requestIdRef.current === requestId;
        if (isMountedRef.current && isLatest) {
          setLoading(false);
        }
      }
    };

    fetchDuel();

    return () => {
      controller.abort();
    };
  }, [duelId]);

  return useMemo(
    () => ({
      data,
      loading,
      error,
    }),
    [data, loading, error]
  );
}
