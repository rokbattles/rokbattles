"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { DateInput } from "@/lib/datetime";

export interface BattleReportTimespan {
  firstStart: DateInput;
  lastEnd: DateInput;
}

export interface BattleReportEntry {
  hash: string;
  startDate: DateInput;
  selfCommanderId: number;
  selfSecondaryCommanderId: number;
  enemyCommanderId: number;
  enemySecondaryCommanderId: number;
}

export interface BattleReportItem {
  parentHash: string;
  count: number;
  timespan: BattleReportTimespan;
  entry: BattleReportEntry;
}

interface BattleReportsResponse {
  items?: BattleReportItem[];
  count?: number;
  cursor?: string;
}

export function useBattleReports() {
  const [reports, setReports] = useState<BattleReportItem[]>([]);
  const [loading, setLoading] = useState(true);
  const mountedRef = useRef(true);

  const fetchReports = useCallback(async () => {
    if (!mountedRef.current) {
      return;
    }

    setLoading(true);

    try {
      const response = await fetch("/api/v2/reports");

      if (!mountedRef.current) {
        return;
      }

      if (!response.ok) {
        throw new Error(`Failed to fetch battle reports (${response.status})`);
      }

      const payload = (await response.json()) as BattleReportsResponse;
      setReports(Array.isArray(payload.items) ? payload.items : []);
    } catch (err) {
      if (!mountedRef.current) {
        return;
      }

      console.error("Failed to fetch battle reports", err);
      setReports([]);
    } finally {
      if (mountedRef.current) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    void fetchReports();

    return () => {
      mountedRef.current = false;
    };
  }, [fetchReports]);

  return { reports, loading, refresh: fetchReports };
}
