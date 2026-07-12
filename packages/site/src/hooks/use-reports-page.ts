"use client";

import { useExtracted } from "next-intl";
import { parseAsString, useQueryState } from "nuqs";
import { use, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { buildReportsQueryParams } from "@/lib/reports-query";
import type { ReportsListResponse } from "@/lib/types/reports-list";
import { GovernorContext } from "@/providers/governor-context";
import { ReportsFilterContext } from "@/providers/reports-filter-context";

export type ReportsScope = "all" | "mine";

type CursorRequest = {
  after?: string;
  before?: string;
};

export type UseReportsPageResult = {
  data: ReportsListResponse["items"];
  loading: boolean;
  error: string | null;
  nextAfter: string | null;
  previousBefore: string | null;
  loadNextPage: () => Promise<void>;
  loadPreviousPage: () => Promise<void>;
};

export function useReportsPage(scope: ReportsScope = "all"): UseReportsPageResult {
  const t = useExtracted();
  const context = use(ReportsFilterContext);
  const governorContext = use(GovernorContext);

  if (!context) {
    throw new Error("useReportsPage must be used within a ReportsFilterProvider");
  }

  if (scope === "mine" && !governorContext) {
    throw new Error("useReportsPage must be used within a GovernorProvider when scope is mine");
  }

  const {
    playerId: filterPlayerId,
    type,
    subtype,
    senderPrimaryCommanderId,
    senderSecondaryCommanderId,
    opponentPrimaryCommanderId,
    opponentSecondaryCommanderId,
    rallySide,
    garrisonSide,
    garrisonBuildingType,
  } = context;

  const playerId = scope === "mine" ? governorContext?.activeGovernor?.governorId : filterPlayerId;

  const [data, setData] = useState<ReportsListResponse["items"]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [nextAfter, setNextAfter] = useState<string | null>(null);
  const [previousBefore, setPreviousBefore] = useState<string | null>(null);
  const [afterParam, setAfterParam] = useQueryState("after", parseAsString);
  const [beforeParam, setBeforeParam] = useQueryState("before", parseAsString);
  const previousFilterSignatureRef = useRef<string | null>(null);
  const requestIdRef = useRef(0);

  const filterSignature = useMemo(
    () =>
      JSON.stringify({
        scope,
        playerId: playerId ?? null,
        type: type ?? null,
        subtype: subtype ?? null,
        senderPrimaryCommanderId: senderPrimaryCommanderId ?? null,
        senderSecondaryCommanderId: senderSecondaryCommanderId ?? null,
        opponentPrimaryCommanderId: opponentPrimaryCommanderId ?? null,
        opponentSecondaryCommanderId: opponentSecondaryCommanderId ?? null,
        rallySide,
        garrisonSide,
        garrisonBuildingType: garrisonBuildingType ?? null,
      }),
    [
      scope,
      playerId,
      type,
      subtype,
      senderPrimaryCommanderId,
      senderSecondaryCommanderId,
      opponentPrimaryCommanderId,
      opponentSecondaryCommanderId,
      rallySide,
      garrisonSide,
      garrisonBuildingType,
    ]
  );

  const fetchPage = useCallback(
    async ({ after, before }: CursorRequest = {}) => {
      const query = buildReportsQueryParams({
        after,
        before,
        playerId,
        type,
        subtype,
        senderPrimaryCommanderId,
        senderSecondaryCommanderId,
        opponentPrimaryCommanderId,
        opponentSecondaryCommanderId,
        rallySide,
        garrisonSide,
        garrisonBuildingType,
      });

      const response = await fetch(`/proxy/v1/reports/battle${query}`, {
        cache: "no-store",
      });

      if (!response.ok) {
        throw new Error(t("Failed to fetch data"));
      }

      return (await response.json()) as ReportsListResponse;
    },
    [
      playerId,
      type,
      subtype,
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
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    if (scope === "mine" && playerId == null) {
      setData([]);
      setError(null);
      setNextAfter(null);
      setPreviousBefore(null);
      setLoading(false);
      return;
    }

    const previousFilterSignature = previousFilterSignatureRef.current;

    if (previousFilterSignature === null) {
      previousFilterSignatureRef.current = filterSignature;
    } else if (previousFilterSignature !== filterSignature) {
      previousFilterSignatureRef.current = filterSignature;

      if (afterParam || beforeParam) {
        void Promise.all([setAfterParam(null), setBeforeParam(null)]);
        return;
      }
    }

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
  }, [
    afterParam,
    beforeParam,
    fetchPage,
    filterSignature,
    playerId,
    scope,
    setAfterParam,
    setBeforeParam,
    t,
  ]);

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
