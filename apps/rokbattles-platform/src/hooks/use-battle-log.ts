"use client";

import { useTranslations } from "next-intl";
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
  const t = useTranslations("errors");
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
          throw new Error(t("battleLog.fetch", { status: res.status }));
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
        const message = err instanceof Error ? err.message : t("battleLog.generic");
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
  }, [governorId, year, t]);

  return {
    data,
    loading,
    error,
  };
}
