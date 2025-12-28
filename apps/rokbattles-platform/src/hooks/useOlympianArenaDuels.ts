"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

export type OlympianArenaParticipant = {
  playerId: number | null;
  playerName: string | null;
  kingdom: number | null;
  alliance: string | null;
  duelId: number | null;
  avatarUrl: string | null;
  frameUrl: string | null;
  primaryCommanderId: number | null;
  secondaryCommanderId: number | null;
};

export type OlympianArenaDuelSummary = {
  duelId: number;
  count: number;
  winStreak: number;
  emailTime: number;
  entry: {
    sender: OlympianArenaParticipant;
    opponent: OlympianArenaParticipant;
  };
};

type OlympianArenaApiResponse = {
  items: OlympianArenaDuelSummary[];
  count: number;
  cursor?: string;
};

type FetchOptions = {
  cursor?: string;
  replace: boolean;
};

export type UseOlympianArenaDuelsResult = {
  data: OlympianArenaDuelSummary[];
  loading: boolean;
  error: string | null;
  cursor: string | undefined;
  loadMore: () => Promise<void>;
};

function buildQueryParams(cursor: string | undefined) {
  const params = new URLSearchParams();

  if (cursor) {
    params.set("cursor", cursor);
  }

  const query = params.toString();
  return query ? `?${query}` : "";
}

export function useOlympianArenaDuels(): UseOlympianArenaDuelsResult {
  const [duels, setDuels] = useState<OlympianArenaDuelSummary[]>([]);
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

  const fetchDuels = useCallback(async ({ cursor, replace }: FetchOptions) => {
    if (loadingRef.current && !replace) {
      return;
    }

    const requestId = ++requestIdRef.current;
    loadingRef.current = true;
    setLoading(true);

    if (replace) {
      setError(null);
    }

    const query = buildQueryParams(cursor);

    try {
      const res = await fetch(`/api/v2/olympian-arena/duels${query}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(`Failed to fetch duels: ${res.status}`);
      }

      const data = (await res.json()) as OlympianArenaApiResponse;
      const isLatest = requestIdRef.current === requestId;
      if (!isMountedRef.current || !isLatest) {
        return;
      }

      const nextCursor = data.cursor ?? undefined;
      cursorRef.current = nextCursor;
      setCursor(nextCursor);
      setDuels((prev) => (replace ? data.items : [...prev, ...data.items]));
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
  }, []);

  useEffect(() => {
    cursorRef.current = undefined;
    setCursor(undefined);
    setDuels([]);
    setError(null);

    fetchDuels({ replace: true });
  }, [fetchDuels]);

  const loadMore = useCallback(async () => {
    if (loadingRef.current) {
      return;
    }

    const nextCursor = cursorRef.current;
    if (!nextCursor) {
      return;
    }

    await fetchDuels({ cursor: nextCursor, replace: false });
  }, [fetchDuels]);

  return useMemo<UseOlympianArenaDuelsResult>(
    () => ({
      data: duels,
      loading,
      error,
      cursor,
      loadMore,
    }),
    [duels, loading, error, cursor, loadMore]
  );
}
