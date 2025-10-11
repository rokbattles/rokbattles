"use client";

import { useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";
import {
  ReportsFilterContext,
  type ReportsFilterType,
} from "@/components/context/ReportsFilterContext";
import type { ReportSummary, UseReportsResult } from "@/hooks/useReports";

type ReportsApiResponse = {
  items: ReportSummary[];
  count: number;
  cursor?: string;
};

type FetchOptions = {
  cursor?: string;
  replace: boolean;
};

function buildQueryParams(
  cursor: string | undefined,
  playerId: number | undefined,
  type: ReportsFilterType | undefined
) {
  const params = new URLSearchParams();

  if (cursor) params.set("cursor", cursor);
  if (typeof playerId === "number" && Number.isFinite(playerId)) {
    params.set("playerId", String(Math.trunc(playerId)));
  }
  if (type) params.set("type", type);

  const query = params.toString();
  return query ? `?${query}` : "";
}

export function useMyReports(): UseReportsResult {
  const filterContext = useContext(ReportsFilterContext);

  if (!filterContext) {
    throw new Error("useMyReports must be used within a ReportsFilterProvider");
  }

  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("useMyReports must be used within a GovernorProvider");
  }

  const { type } = filterContext;
  const { activeGovernor } = governorContext;
  const playerId = activeGovernor?.governorId;

  const [reports, setReports] = useState<ReportSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [cursor, setCursor] = useState<string | undefined>(undefined);

  const cursorRef = useRef<string | undefined>(undefined);
  const loadingRef = useRef(false);
  const requestIdRef = useRef(0);
  const isMountedRef = useRef(true);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  const fetchReports = useCallback(
    async ({ cursor, replace }: FetchOptions) => {
      if (loadingRef.current && !replace) {
        return;
      }

      if (playerId == null) {
        if (replace) {
          loadingRef.current = false;
          setLoading(false);
          setError(null);
          cursorRef.current = undefined;
          setCursor(undefined);
          setReports([]);
        }
        return;
      }

      const requestId = ++requestIdRef.current;
      loadingRef.current = true;
      setLoading(true);

      if (replace) {
        setError(null);
      }

      const query = buildQueryParams(cursor, playerId, type);

      try {
        const res = await fetch(`/api/v2/reports${query}`, {
          cache: "no-store",
        });

        if (!res.ok) {
          throw new Error(`Failed to fetch reports: ${res.status}`);
        }

        const data = (await res.json()) as ReportsApiResponse;
        const isLatest = requestIdRef.current === requestId;
        if (!isMountedRef.current || !isLatest) {
          return;
        }

        const nextCursor = data.cursor ?? undefined;
        cursorRef.current = nextCursor;
        setCursor(nextCursor);
        setReports((prev) => (replace ? data.items : [...prev, ...data.items]));
        setError(null);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        const isLatest = requestIdRef.current === requestId;

        if (!isMountedRef.current || !isLatest) {
          return;
        }
        setError(message);
      } finally {
        const isLatest = requestIdRef.current === requestId;

        if (isLatest) {
          loadingRef.current = false;
        }
        if (isLatest && isMountedRef.current) {
          setLoading(false);
        }
      }
    },
    [playerId, type]
  );

  useEffect(() => {
    cursorRef.current = undefined;
    setCursor(undefined);
    setReports([]);
    setError(null);

    if (playerId == null) {
      loadingRef.current = false;
      setLoading(false);
      return;
    }

    fetchReports({ replace: true });
  }, [fetchReports, playerId]);

  const loadMore = useCallback(async () => {
    if (loadingRef.current || playerId == null) {
      return;
    }

    const nextCursor = cursorRef.current;
    if (!nextCursor) {
      return;
    }

    await fetchReports({ cursor: nextCursor, replace: false });
  }, [fetchReports, playerId]);

  return useMemo<UseReportsResult>(
    () => ({
      data: reports,
      loading,
      error,
      cursor,
      loadMore,
    }),
    [reports, loading, error, cursor, loadMore]
  );
}
