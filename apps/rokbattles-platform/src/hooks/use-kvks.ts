"use client";

import { useTranslations } from "next-intl";
import { useEffect, useState } from "react";

export type KvkSummary = {
  serverId: number;
  reportCount: number;
};

export type KvksResponse = {
  items: KvkSummary[];
  count: number;
};

export function useKvks(governorId: number | null | undefined) {
  const t = useTranslations("errors");
  const [data, setData] = useState<KvksResponse | null>(null);
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

    fetch(`/api/v2/governor/${governorId}/kvks`, { cache: "no-store" })
      .then((res) => {
        if (!res.ok) {
          throw new Error(t("kvks.fetch", { status: res.status }));
        }
        return res.json() as Promise<KvksResponse>;
      })
      .then((payload) => {
        if (!cancelled) {
          setData(payload);
        }
      })
      .catch((err: unknown) => {
        if (cancelled) {
          return;
        }
        const message = err instanceof Error ? err.message : t("kvks.generic");
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
  }, [governorId, t]);

  return { data, loading, error };
}
