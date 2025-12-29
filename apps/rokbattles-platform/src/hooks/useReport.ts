"use client";

import { useEffect, useState } from "react";
import type { ReportByHashResponse } from "@/lib/types/report";

export function useReport(hash: string | null | undefined) {
  const [data, setData] = useState<ReportByHashResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!hash) {
      setData(null);
      setError(null);
      setLoading(false);
      return;
    }

    let cancelled = false;

    setData(null);
    setLoading(true);
    setError(null);

    const fetchReport = async () => {
      try {
        const res = await fetch(`/api/v2/report/${encodeURIComponent(hash)}`);

        if (!res.ok) {
          throw new Error(`Failed to fetch report: ${res.status}`);
        }

        const payload = (await res.json()) as ReportByHashResponse;
        if (!cancelled) {
          setData(payload);
          setError(null);
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
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

    fetchReport();

    return () => {
      cancelled = true;
    };
  }, [hash]);

  return {
    data,
    loading,
    error,
  };
}
