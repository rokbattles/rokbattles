"use client";

import { useCallback, useEffect, useState } from "react";
import { type DeviceSession, fetchSessions, SessionRequestError } from "@/lib/(app)/sessions";

type SessionsResult = {
  revision: number;
  sessions: DeviceSession[] | null;
  error: string | null;
};

export function useSessions() {
  const [revision, setRevision] = useState(0);
  const [result, setResult] = useState<SessionsResult | null>(null);

  useEffect(() => {
    const controller = new AbortController();

    fetchSessions(controller.signal)
      .then((sessions) => {
        if (!controller.signal.aborted) {
          setResult({ revision, sessions, error: null });
        }
      })
      .catch((error) => {
        if (controller.signal.aborted) {
          return;
        }

        if (error instanceof SessionRequestError && error.status === 401) {
          window.location.replace("/");
          return;
        }

        setResult((previous) => ({
          revision,
          sessions: previous?.sessions ?? null,
          error: "Unable to load logged-in devices. Please try again.",
        }));
      });

    return () => controller.abort();
  }, [revision]);

  const refresh = useCallback(() => setRevision((value) => value + 1), []);

  return {
    sessions: result?.sessions ?? null,
    error: result?.error ?? null,
    loading: result?.revision !== revision,
    refresh,
  };
}
