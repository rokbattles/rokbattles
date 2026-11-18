"use client";

import { useEffect, useState } from "react";

export function usePairingsQuery<T>(url: string | null) {
  const [attempt, setAttempt] = useState(0);
  const [result, setResult] = useState<{
    key: string;
    data: T | null;
    error: string | null;
  } | null>(null);

  const key = `${url}:${attempt}`;

  useEffect(() => {
    if (!url) return;
    const controller = new AbortController();

    async function load(requestUrl: string) {
      try {
        const response = await fetch(requestUrl, { cache: "no-store", signal: controller.signal });
        if (!response.ok) {
          throw new Error(
            response.status === 401
              ? "Sign in again to view your pairings."
              : "We couldn't load your pairings. Please try again."
          );
        }

        const data = (await response.json()) as T;
        if (!controller.signal.aborted) setResult({ key, data, error: null });
      } catch (error) {
        if (!controller.signal.aborted) {
          setResult({
            key,
            data: null,
            error: error instanceof Error ? error.message : "Unable to load pairings.",
          });
        }
      }
    }

    void load(url);
    return () => controller.abort();
  }, [url, key]);

  const current = url && result?.key === key ? result : null;

  return {
    data: current?.data ?? null,
    loading: Boolean(url && !current),
    error: current?.error ?? null,
    retry: () => setAttempt((value) => value + 1),
  };
}
