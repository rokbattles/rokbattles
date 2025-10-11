"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import type { ReportByHashResponse } from "@/lib/types/report";

export function useReport(hash: string | null | undefined) {
  const [data, setData] = useState<ReportByHashResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const requestIdRef = useRef(0);
  const isMountedRef = useRef(true);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (!hash) {
      setData(null);
      setError(null);
      setLoading(false);
      return;
    }

    const requestId = ++requestIdRef.current;
    const controller = new AbortController();

    setData(null);
    setLoading(true);
    setError(null);

    const fetchReport = async () => {
      try {
        const res = await fetch(`/api/v2/report/${encodeURIComponent(hash)}`, {
          signal: controller.signal,
        });

        if (!res.ok) {
          throw new Error(`Failed to fetch report: ${res.status}`);
        }

        const payload = (await res.json()) as ReportByHashResponse;

        const isLatest = requestIdRef.current === requestId;
        if (!isMountedRef.current || !isLatest) {
          return;
        }

        setData(payload);
        setError(null);
      } catch (err) {
        if (controller.signal.aborted) {
          return;
        }

        const isLatest = requestIdRef.current === requestId;
        if (!isMountedRef.current || !isLatest) {
          return;
        }

        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setData(null);
      } finally {
        const isLatest = requestIdRef.current === requestId;
        if (isMountedRef.current && isLatest) {
          setLoading(false);
        }
      }
    };

    fetchReport();

    return () => {
      controller.abort();
    };
  }, [hash]);

  return useMemo(
    () => ({
      data,
      loading,
      error,
    }),
    [data, loading, error]
  );
}
