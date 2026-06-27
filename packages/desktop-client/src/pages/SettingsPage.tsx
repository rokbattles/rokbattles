import type { ReactNode } from "react";

import { Switch } from "../components/Switch.tsx";
import { useAppSettings } from "../hooks/useAppSettings.ts";
import { parseCloseBehavior } from "../lib/close-behavior.ts";

type ToggleValue = "enabled" | "disabled";

const toggleOptions = [
  { label: "Enabled", value: "enabled" },
  { label: "Disabled", value: "disabled" },
] satisfies Array<{ label: string; value: ToggleValue }>;

function toggleValue(enabled: boolean): ToggleValue {
  return enabled ? "enabled" : "disabled";
}

export function SettingsPage(): ReactNode {
  const {
    settings,
    isLoading,
    pendingSetting,
    updateAutoUpdate,
    updateAutoStart,
    updateCloseBehavior,
  } = useAppSettings();

  const closeBehavior = settings?.close_behavior ?? "minimize_to_tray";
  const traySupported = settings?.tray_supported ?? true;
  const effectiveCloseBehavior =
    traySupported || closeBehavior !== "minimize_to_tray" ? closeBehavior : "quit";
  const isAutoUpdatePending = pendingSetting === "auto_update";
  const isAutoStartPending = pendingSetting === "auto_start";
  const isCloseBehaviorPending = pendingSetting === "close_behavior";
  const closeBehaviorOptions = [
    { disabled: !traySupported, label: "Minimize to tray", value: "minimize_to_tray" },
    { label: "Quit", value: "quit" },
  ];

  return (
    <section className="py-6">
      {isLoading ? (
        <div className="py-6 text-sm text-zinc-400">Loading settings...</div>
      ) : (
        <div className="space-y-3">
          <div className="flex items-center justify-between gap-4 py-2">
            <span className="text-sm/6 font-medium text-white">Auto update</span>
            <Switch
              label="Auto update"
              value={toggleValue(settings?.auto_update ?? true)}
              options={toggleOptions}
              disabled={isAutoUpdatePending}
              onChange={(value) => updateAutoUpdate(value === "enabled")}
            />
          </div>

          <div className="flex items-center justify-between gap-4 py-2">
            <span className="text-sm/6 font-medium text-white">Auto start</span>
            <Switch
              label="Auto start"
              value={toggleValue(settings?.auto_start ?? true)}
              options={toggleOptions}
              disabled={isAutoStartPending}
              onChange={(value) => updateAutoStart(value === "enabled")}
            />
          </div>

          <div className="flex items-center justify-between gap-4 py-2">
            <span className="text-sm/6 font-medium text-white">Close prompt</span>
            <Switch
              label="Close prompt"
              value={effectiveCloseBehavior}
              options={closeBehaviorOptions}
              disabled={isCloseBehaviorPending}
              onChange={(value) => updateCloseBehavior(parseCloseBehavior(value))}
            />
          </div>
        </div>
      )}
    </section>
  );
}
