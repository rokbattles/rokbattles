"use client";

import { useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import {
  ReportsFilterContext,
  type ReportsFilterType,
} from "@/components/context/ReportsFilterContext";

export type ReportSummaryEntry = {
  hash: string;
  startDate: number;
  selfCommanderId: number;
  selfSecondaryCommanderId: number;
  enemyCommanderId: number;
  enemySecondaryCommanderId: number;
};

export type ReportSummary = {
  parentHash: string;
  count: number;
  timespan: {
    firstStart: number;
    lastEnd: number;
  };
  entry: ReportSummaryEntry;
};

type ReportsApiResponse = {
  items: ReportSummary[];
  count: number;
  cursor?: string;
};

type FetchOptions = {
  cursor?: string;
  replace: boolean;
};

export type UseReportsResult = {
  data: ReportSummary[];
  loading: boolean;
  error: string | null;
  cursor: string | undefined;
  loadMore: () => Promise<void>;
};

function buildQueryParams(
  cursor: string | undefined,
  playerId: number | undefined,
  type: ReportsFilterType | undefined,
  commanderId: number | undefined
) {
  const params = new URLSearchParams();

  if (cursor) params.set("cursor", cursor);
  if (typeof playerId === "number" && Number.isFinite(playerId)) {
    params.set("playerId", String(Math.trunc(playerId)));
  }
  if (type) params.set("type", type);
  if (typeof commanderId === "number" && Number.isFinite(commanderId) && commanderId > 0) {
    params.set("commanderId", String(Math.trunc(commanderId)));
  }

  const query = params.toString();
  return query ? `?${query}` : "";
}

export function useReports(): UseReportsResult {
  const context = useContext(ReportsFilterContext);

  if (!context) {
    throw new Error("useReports must be used within a ReportsFilterProvider");
  }

  const { playerId, type, commanderId } = context;

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

      const requestId = ++requestIdRef.current;
      loadingRef.current = true;
      setLoading(true);

      if (replace) {
        setError(null);
      }

      const query = buildQueryParams(cursor, playerId, type, commanderId);
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
    [playerId, type, commanderId]
  );

  useEffect(() => {
    cursorRef.current = undefined;
    setCursor(undefined);
    setReports([]);
    setError(null);

    fetchReports({ replace: true });
  }, [fetchReports]);

  const loadMore = useCallback(async () => {
    if (loadingRef.current) {
      return;
    }

    const nextCursor = cursorRef.current;
    if (!nextCursor) {
      return;
    }

    await fetchReports({ cursor: nextCursor, replace: false });
  }, [fetchReports]);

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
