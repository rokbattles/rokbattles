"use client";

import { SettingsLink } from "@/components/(app)/account/settings/settings-link";
import { useSessions } from "@/hooks/(app)/use-sessions";

export function LoggedInDevicesLink() {
  const { sessions, loading } = useSessions();
  const count = sessions?.length;
  const value = loading
    ? "…"
    : count == null
      ? "View devices"
      : `${count} ${count === 1 ? "device" : "devices"}`;

  return <SettingsLink href="/account/settings/devices" label="Logged-in Devices" value={value} />;
}
