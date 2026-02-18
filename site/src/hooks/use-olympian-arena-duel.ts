"use client";

import { useExtracted } from "next-intl";
import { useEffect, useState } from "react";
import type { DuelBattle2MailDocument } from "@/lib/types/duelbattle2";

export type DuelReportEntry = DuelBattle2MailDocument;

export type DuelReportResponse = {
  duelId: number;
  items: DuelReportEntry[];
  count: number;
};

export function useOlympianArenaDuel(duelId: number | null | undefined) {
  const t = useExtracted();
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
          throw new Error(t("Failed to fetch data"));
        }

        const payload = (await res.json()) as DuelReportResponse;
        if (!cancelled) {
          setData(payload);
          setError(null);
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : t("Failed to fetch data");
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
  }, [duelId, t]);

  return {
    data,
    loading,
    error,
  };
}
