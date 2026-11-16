"use client";

import { useState } from "react";
import {
  disableOverlay,
  type OverlayConfig,
  rotateOverlayUrl,
  saveOverlaySettings,
} from "@/lib/(app)/stream-overlay";
import type { OverlayUrlAction } from "@/lib/(app)/types/overlay";

export function useOverlayForm(governorId: number, initialConfig: OverlayConfig) {
  const [config, setConfig] = useState(initialConfig);
  const [settings, setSettings] = useState(initialConfig.settings);
  const [pending, setPending] = useState<"save" | OverlayUrlAction | null>(null);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");

  const dirty =
    settings.limit !== config.settings.limit || settings.branding !== config.settings.branding;
  const url = config.path ? `https://rokbattles.com${config.path}` : null;

  async function rotate(): Promise<void> {
    const { path } = await rotateOverlayUrl(governorId);

    setConfig((previous) => ({ ...previous, path, enabled: true }));
  }

  async function save(): Promise<void> {
    setPending("save");
    setError("");
    setMessage("");

    try {
      const saved = await saveOverlaySettings(governorId, settings);

      setConfig((previous) => ({ ...previous, settings: saved }));

      if (!config.enabled) {
        await rotate();
        setMessage("Overlay URL created and settings saved.");
      } else {
        setMessage("Settings saved.");
      }
    } catch {
      setError("Unable to save overlay settings or create the URL. Please try again.");
    } finally {
      setPending(null);
    }
  }

  async function changeUrl(action: OverlayUrlAction): Promise<void> {
    setPending(action);
    setError("");
    setMessage("");

    try {
      if (action === "rotate") {
        await rotate();
        setMessage("URL rotated. Update the browser source in your streaming software.");
      } else {
        await disableOverlay(governorId);

        setConfig((previous) => ({ ...previous, enabled: false, path: null }));
        setMessage("Overlay disabled. You can create a new URL whenever you’re ready.");
      }
    } catch {
      setError("Unable to update the overlay URL. Please try again.");
    } finally {
      setPending(null);
    }
  }

  return {
    settings,
    setSettings,
    enabled: config.enabled,
    url,
    dirty,
    pending,
    error,
    message,
    save,
    changeUrl,
  };
}
