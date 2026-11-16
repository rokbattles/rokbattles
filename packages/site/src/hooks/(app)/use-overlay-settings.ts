"use client";

import { useEffect, useState } from "react";
import { fetchOverlayConfig, type OverlayConfig } from "@/lib/(app)/stream-overlay";

export function useOverlaySettings(governorId: number) {
  const [config, setConfig] = useState<OverlayConfig | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    const controller = new AbortController();

    fetchOverlayConfig(governorId, controller.signal)
      .then((value) => {
        if (!controller.signal.aborted) {
          setConfig(value);
        }
      })
      .catch(() => {
        if (!controller.signal.aborted) {
          setError("Unable to load overlay settings.");
        }
      });

    return () => controller.abort();
  }, [governorId]);

  return { config, error };
}
