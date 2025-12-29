"use client";

import { useEffect, useState } from "react";

export type BattleLogDay = {
  date: string;
  battleCount: number;
  npcCount: number;
};

export type BattleLogResponse = {
  startDate: string;
  endDate: string;
  days: BattleLogDay[];
};

export function useBattleLog(governorId: number | null | undefined, year?: number) {
  const [data, setData] = useState<BattleLogResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (governorId == null) {
      setData(null);
      setError(null);
      setLoading(false);
      return;
    }

    let cancelled = false;

    setLoading(true);
    setError(null);

    fetch(`/api/v2/governor/${governorId}/battle-log?year=${String(year)}`, {
      cache: "no-store",
    })
      .then((res) => {
        if (!res.ok) {
          throw new Error(`Failed to load battle log (${res.status})`);
        }
        return res.json() as Promise<BattleLogResponse>;
      })
      .then((json) => {
        if (!cancelled) {
          setData(json);
        }
      })
      .catch((err: unknown) => {
        if (cancelled) {
          return;
        }
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setData(null);
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [governorId, year]);

  return {
    data,
    loading,
    error,
  };
}
