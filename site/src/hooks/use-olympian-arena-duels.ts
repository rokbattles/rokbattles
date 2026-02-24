"use client";

import { useExtracted } from "next-intl";
import { parseAsString, useQueryState } from "nuqs";
import { useCallback, useEffect, useRef, useState } from "react";

export type OlympianArenaParticipant = {
  primaryCommanderId: number | null;
  secondaryCommanderId: number | null;
};

export type OlympianArenaDuelSummary = {
  duelId: number;
  winStreak: number;
  mailTime: number;
  killCount: number;
  tradePercent: number;
  entry: {
    sender: OlympianArenaParticipant;
    opponent: OlympianArenaParticipant;
  };
};

type OlympianArenaApiResponse = {
  items: OlympianArenaDuelSummary[];
  nextAfter: string | null;
  previousBefore: string | null;
};

type CursorRequest = {
  after?: string;
  before?: string;
};

export type UseOlympianArenaDuelsResult = {
  data: OlympianArenaDuelSummary[];
  loading: boolean;
  error: string | null;
  nextAfter: string | null;
  previousBefore: string | null;
  loadNextPage: () => Promise<void>;
  loadPreviousPage: () => Promise<void>;
};

function buildQueryParams({ after, before }: CursorRequest = {}) {
  const params = new URLSearchParams();

  if (before) {
    params.set("before", before);
  } else if (after) {
    params.set("after", after);
  }

  const query = params.toString();
  return query ? `?${query}` : "";
}

export function useOlympianArenaDuels(): UseOlympianArenaDuelsResult {
  const t = useExtracted();
  const [data, setData] = useState<OlympianArenaDuelSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [nextAfter, setNextAfter] = useState<string | null>(null);
  const [previousBefore, setPreviousBefore] = useState<string | null>(null);
  const [afterParam, setAfterParam] = useQueryState("after", parseAsString);
  const [beforeParam, setBeforeParam] = useQueryState("before", parseAsString);
  const requestIdRef = useRef(0);

  const fetchPage = useCallback(
    async ({ after, before }: CursorRequest = {}) => {
      const query = buildQueryParams({ after, before });
      const res = await fetch(`/proxy/v1/reports/duelbattle2${query}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(t("Failed to fetch data"));
      }

      return (await res.json()) as OlympianArenaApiResponse;
    },
    [t]
  );

  useEffect(() => {
    let cancelled = false;
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    setLoading(true);

    fetchPage(beforeParam ? { before: beforeParam } : afterParam ? { after: afterParam } : {})
      .then((payload) => {
        if (cancelled || requestId !== requestIdRef.current) {
          return;
        }

        setData(payload.items);
        setNextAfter(payload.nextAfter);
        setPreviousBefore(payload.previousBefore);
        setError(null);
      })
      .catch((err) => {
        if (cancelled || requestId !== requestIdRef.current) {
          return;
        }

        const message = err instanceof Error ? err.message : t("Failed to fetch data");
        setError(message);
        setData([]);
        setNextAfter(null);
        setPreviousBefore(null);
      })
      .finally(() => {
        if (cancelled || requestId !== requestIdRef.current) {
          return;
        }

        setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [afterParam, beforeParam, fetchPage, t]);

  const loadNextPage = useCallback(async () => {
    if (!nextAfter || loading) {
      return;
    }

    setLoading(true);
    setError(null);
    try {
      await Promise.all([setBeforeParam(null), setAfterParam(nextAfter)]);
    } catch (err) {
      const message = err instanceof Error ? err.message : t("Failed to fetch data");
      setError(message);
      setLoading(false);
    }
  }, [loading, nextAfter, setAfterParam, setBeforeParam, t]);

  const loadPreviousPage = useCallback(async () => {
    if (!previousBefore || loading) {
      return;
    }

    setLoading(true);
    setError(null);
    try {
      await Promise.all([setAfterParam(null), setBeforeParam(previousBefore)]);
    } catch (err) {
      const message = err instanceof Error ? err.message : t("Failed to fetch data");
      setError(message);
      setLoading(false);
    }
  }, [loading, previousBefore, setAfterParam, setBeforeParam, t]);

  return {
    data,
    loading,
    error,
    nextAfter,
    previousBefore,
    loadNextPage,
    loadPreviousPage,
  };
}
