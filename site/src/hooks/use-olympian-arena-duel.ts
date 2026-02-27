"use client";

import { useExtracted } from "next-intl";
import { useEffect, useState } from "react";
import type { DuelBattle2DetailItem, DuelBattle2DetailResponse } from "@/lib/types/duelbattle2";

export type DuelReportEntry = DuelBattle2DetailItem;

export function useOlympianArenaDuel(duelId: number | null | undefined) {
  const t = useExtracted();
  const [data, setData] = useState<DuelBattle2DetailResponse | null>(null);
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
        const res = await fetch(`/proxy/v1/report/duelbattle2/${encodeURIComponent(duelId)}`, {
          cache: "no-store",
        });

        if (!res.ok) {
          throw new Error(t("Failed to fetch data"));
        }

        const payload = (await res.json()) as DuelBattle2DetailResponse;
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
