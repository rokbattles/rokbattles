"use client";

import { useCallback, useEffect, useState } from "react";
import { fetchProgramApplications } from "@/lib/(app)/program-applications";
import type { ProgramApplication } from "@/lib/(app)/types/program-application";

export function useProgramApplications() {
  const [applications, setApplications] = useState<ProgramApplication[] | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async (signal?: AbortSignal) => {
    try {
      const applications = await fetchProgramApplications(signal);

      if (!signal?.aborted) setApplications(applications);
    } catch {
      if (!signal?.aborted) setApplications(null);
    } finally {
      if (!signal?.aborted) setLoading(false);
    }
  }, []);

  useEffect(() => {
    const controller = new AbortController();

    void load(controller.signal);

    return () => controller.abort();
  }, [load]);

  async function refresh() {
    setLoading(true);
    await load();
  }

  function recordApplication(application: ProgramApplication) {
    setApplications((current) => [
      ...(current ?? []).filter((item) => item.program !== application.program),
      application,
    ]);
  }

  return {
    applications: applications ?? [],
    statusAvailable: applications !== null,
    loading,
    refresh,
    recordApplication,
  };
}
