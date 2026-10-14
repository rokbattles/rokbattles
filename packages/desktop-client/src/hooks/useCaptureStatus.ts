import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import { type CaptureStatus, getCaptureStatus } from "../lib/tauri-client.ts";

export function useCaptureStatus(): CaptureStatus | null {
  const [status, setStatus] = useState<CaptureStatus | null>(null);

  useEffect(() => {
    let disposed = false;
    let revision = 0;

    const refresh = async () => {
      const current = ++revision;

      try {
        const next = await getCaptureStatus();
        if (!disposed && current === revision) setStatus(next);
      } catch (error) {
        console.error("Failed to read capture status", error);
        if (!disposed && current === revision) setStatus("failed");
      }
    };

    // Listen before reading so a change during initialization cannot be missed.
    const subscription = listen("capture-status-changed", refresh);
    subscription.then(refresh).catch((error) => {
      console.error("Failed to listen for capture status", error);
      void refresh();
    });

    return () => {
      disposed = true;
      subscription.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);

  return status;
}
