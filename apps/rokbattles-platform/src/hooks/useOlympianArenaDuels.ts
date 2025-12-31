"use client";

import { useTranslations } from "next-intl";
import { useCallback, useEffect, useState } from "react";

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
  const t = useTranslations("errors");
  const [duels, setDuels] = useState<OlympianArenaDuelSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [cursor, setCursor] = useState<string | undefined>(undefined);

  const fetchDuels = useCallback(
    async (nextCursor?: string) => {
      const query = buildQueryParams(nextCursor);

      const res = await fetch(`/api/v2/olympian-arena/duels${query}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(t("duels.fetch", { status: res.status }));
      }

      return (await res.json()) as OlympianArenaApiResponse;
    },
    [t]
  );

  useEffect(() => {
    let cancelled = false;

    setCursor(undefined);
    setDuels([]);
    setError(null);
    setLoading(true);

    fetchDuels()
      .then((data) => {
        if (cancelled) {
          return;
        }
        setCursor(data.cursor ?? undefined);
        setDuels(data.items);
        setError(null);
      })
      .catch((err) => {
        if (cancelled) {
          return;
        }
        const message = err instanceof Error ? err.message : t("duels.generic");
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
  }, [fetchDuels, t]);

  const loadMore = async () => {
    if (loading) {
      return;
    }

    if (!cursor) {
      return;
    }

    setLoading(true);

    try {
      const data = await fetchDuels(cursor);
      setCursor(data.cursor ?? undefined);
      setDuels((prev) => [...prev, ...data.items]);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : t("duels.generic");
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  return {
    data: duels,
    loading,
    error,
    cursor,
    loadMore,
  };
}
