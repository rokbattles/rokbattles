"use client";

import { useEffect, useState } from "react";

type ArkQueryResult<T> = {
  data: T | null;
  loading: boolean;
  error: string | null;
  retry: () => void;
};

type ArkQueryState<T> = {
  key: string;
  data: T | null;
  error: string | null;
};

export function useArkQuery<T>(url: string | null): ArkQueryResult<T> {
  const [attempt, setAttempt] = useState(0);
  const [result, setResult] = useState<ArkQueryState<T> | null>(null);

  const key = `${url}:${attempt}`;

  useEffect(() => {
    if (!url) {
      return;
    }

    const controller = new AbortController();

    async function load(requestUrl: string): Promise<void> {
      try {
        const response = await fetch(requestUrl, {
          cache: "no-store",
          signal: controller.signal,
        });

        if (!response.ok) {
          throw new Error(`Failed to load Ark recap: ${response.status}`);
        }

        const data = (await response.json()) as T;

        if (!controller.signal.aborted) {
          setResult({ key, data, error: null });
        }
      } catch (error) {
        if (!controller.signal.aborted) {
          setResult({
            key,
            data: null,
            error: error instanceof Error ? error.message : String(error),
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
    loading: !!url && !current,
    error: current?.error ?? null,
    retry: () => setAttempt((value) => value + 1),
  };
}
