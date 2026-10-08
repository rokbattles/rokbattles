"use client";

import { useEffect, useState } from "react";
import type { ResourcesQueryResult } from "@/lib/types/resources";

type ResourcesOptions = {
  governorId: number | null | undefined;
  startParam: string;
  endParam: string;
};

type ResourcesResult = {
  data: ResourcesQueryResult | null;
  loading: boolean;
  error: string | null;
  retry: () => void;
};

type ResourcesRequestResult = {
  key: string;
  data: ResourcesQueryResult | null;
  error: string | null;
};

export function useResources({
  governorId,
  startParam,
  endParam,
}: ResourcesOptions): ResourcesResult {
  const [result, setResult] = useState<ResourcesRequestResult | null>(null);
  const [attempt, setAttempt] = useState(0);
  const hasGovernor = governorId != null && Number.isFinite(governorId);
  const key = `${governorId}:${startParam}:${endParam}:${attempt}`;

  useEffect(() => {
    if (!hasGovernor) {
      return;
    }

    const controller = new AbortController();

    async function fetchResources(): Promise<void> {
      try {
        const params = new URLSearchParams({ start: startParam, end: endParam });
        const response = await fetch(`/proxy/v1/governor/${governorId}/resources?${params}`, {
          cache: "no-store",
          signal: controller.signal,
        });

        if (!response.ok) {
          throw new Error(`Failed to load resources: ${response.status}`);
        }

        const data = (await response.json()) as ResourcesQueryResult;

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

    void fetchResources();

    return () => controller.abort();
  }, [governorId, hasGovernor, startParam, endParam, key]);

  const current = hasGovernor && result?.key === key ? result : null;

  return {
    data: current?.data ?? null,
    loading: hasGovernor && !current,
    error: current?.error ?? null,
    retry: () => setAttempt((value) => value + 1),
  };
}
