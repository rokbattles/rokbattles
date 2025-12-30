"use client";

import { useTranslations } from "next-intl";
import { useCallback, useContext, useEffect, useState } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";
import { ReportsFilterContext } from "@/components/context/ReportsFilterContext";
import { buildReportsQueryParams } from "@/lib/reportsQuery";

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

export type UseReportsResult = {
  data: ReportSummary[];
  loading: boolean;
  error: string | null;
  cursor: string | undefined;
  loadMore: () => Promise<void>;
};

export type ReportsScope = "all" | "mine";

export type UseReportsOptions = {
  scope?: ReportsScope;
};

export function useReports({ scope = "all" }: UseReportsOptions = {}): UseReportsResult {
  const t = useTranslations("errors");
  const context = useContext(ReportsFilterContext);
  const governorContext = useContext(GovernorContext);

  if (!context) {
    throw new Error("useReports must be used within a ReportsFilterProvider");
  }

  if (scope === "mine" && !governorContext) {
    throw new Error("useReports must be used within a GovernorProvider when scope is mine");
  }

  const {
    playerId: filterPlayerId,
    type,
    senderPrimaryCommanderId,
    senderSecondaryCommanderId,
    opponentPrimaryCommanderId,
    opponentSecondaryCommanderId,
    rallySide,
    garrisonSide,
    garrisonBuildingType,
  } = context;
  const playerId = scope === "mine" ? governorContext?.activeGovernor?.governorId : filterPlayerId;

  const [reports, setReports] = useState<ReportSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [cursor, setCursor] = useState<string | undefined>(undefined);

  const fetchReports = useCallback(
    async (nextCursor?: string) => {
      const query = buildReportsQueryParams({
        cursor: nextCursor,
        playerId,
        type,
        senderPrimaryCommanderId,
        senderSecondaryCommanderId,
        opponentPrimaryCommanderId,
        opponentSecondaryCommanderId,
        rallySide,
        garrisonSide,
        garrisonBuildingType,
      });

      const res = await fetch(`/api/v2/reports${query}`, {
        cache: "no-store",
      });

      if (!res.ok) {
        throw new Error(t("reports.fetch", { status: res.status }));
      }

      return (await res.json()) as ReportsApiResponse;
    },
    [
      playerId,
      type,
      senderPrimaryCommanderId,
      senderSecondaryCommanderId,
      opponentPrimaryCommanderId,
      opponentSecondaryCommanderId,
      rallySide,
      garrisonSide,
      garrisonBuildingType,
      t,
    ]
  );

  useEffect(() => {
    let cancelled = false;

    setCursor(undefined);
    setReports([]);
    setError(null);

    if (scope === "mine" && playerId == null) {
      setLoading(false);
      return;
    }

    setLoading(true);

    fetchReports()
      .then((data) => {
        if (cancelled) {
          return;
        }
        setCursor(data.cursor ?? undefined);
        setReports(data.items);
        setError(null);
      })
      .catch((err) => {
        if (cancelled) {
          return;
        }
        const message = err instanceof Error ? err.message : t("reports.generic");
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
  }, [fetchReports, playerId, scope, t]);

  const loadMore = async () => {
    if (loading) {
      return;
    }

    if (!cursor) {
      return;
    }

    if (scope === "mine" && playerId == null) {
      return;
    }

    setLoading(true);

    try {
      const data = await fetchReports(cursor);
      setCursor(data.cursor ?? undefined);
      setReports((prev) => [...prev, ...data.items]);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : t("reports.generic");
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  return {
    data: reports,
    loading,
    error,
    cursor,
    loadMore,
  };
}
