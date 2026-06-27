import { useCallback, useEffect, useState } from "react";

import { type CloseBehavior, parseCloseBehavior } from "../lib/close-behavior.ts";
import {
  type AppSettings,
  getAppSettings,
  setAutoStart,
  setAutoUpdate,
  setCloseBehavior,
} from "../lib/tauri-client.ts";

type PendingSetting = "auto_update" | "auto_start" | "close_behavior";

type UseAppSettingsResult = {
  settings: AppSettings | null;
  isLoading: boolean;
  pendingSetting: PendingSetting | null;
  updateAutoUpdate: (enabled: boolean) => void;
  updateAutoStart: (enabled: boolean) => void;
  updateCloseBehavior: (behavior: CloseBehavior) => void;
};

export function useAppSettings(): UseAppSettingsResult {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [pendingSetting, setPendingSetting] = useState<PendingSetting | null>(null);

  const loadSettings = useCallback(async () => {
    try {
      setIsLoading(true);
      const nextSettings = await getAppSettings();
      setSettings({
        ...nextSettings,
        close_behavior: parseCloseBehavior(nextSettings.close_behavior),
      });
    } catch (error) {
      console.error("Failed to load settings", error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  const saveSetting = useCallback(
    async (setting: PendingSetting, save: () => Promise<unknown>) => {
      try {
        setPendingSetting(setting);
        await save();
      } catch (error) {
        console.error("Failed to save setting", error);
        await loadSettings();
      } finally {
        setPendingSetting(null);
      }
    },
    [loadSettings]
  );

  const updateAutoUpdate = useCallback(
    (enabled: boolean) => {
      setSettings((current) => (current ? { ...current, auto_update: enabled } : current));
      void saveSetting("auto_update", () => setAutoUpdate(enabled));
    },
    [saveSetting]
  );

  const updateAutoStart = useCallback(
    (enabled: boolean) => {
      setSettings((current) => (current ? { ...current, auto_start: enabled } : current));
      void saveSetting("auto_start", () => setAutoStart(enabled));
    },
    [saveSetting]
  );

  const updateCloseBehavior = useCallback(
    (behavior: CloseBehavior) => {
      setSettings((current) => (current ? { ...current, close_behavior: behavior } : current));
      void saveSetting("close_behavior", () => setCloseBehavior(behavior));
    },
    [saveSetting]
  );

  return {
    settings,
    isLoading,
    pendingSetting,
    updateAutoUpdate,
    updateAutoStart,
    updateCloseBehavior,
  };
}
