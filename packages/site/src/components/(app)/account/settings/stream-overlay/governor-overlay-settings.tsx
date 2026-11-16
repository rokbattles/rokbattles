"use client";

import type { JSX } from "react";
import { OverlayForm } from "@/components/(app)/account/settings/stream-overlay/overlay-form";
import { Text } from "@/components/ui/text";
import { useOverlaySettings } from "@/hooks/(app)/use-overlay-settings";
import type { OverlayCommander } from "@/lib/(app)/types/overlay";

type GovernorOverlaySettingsProps = {
  governorId: number;
  commanders: Record<string, OverlayCommander>;
};

export function GovernorOverlaySettings({
  governorId,
  commanders,
}: GovernorOverlaySettingsProps): JSX.Element {
  const { config, error } = useOverlaySettings(governorId);

  if (error) {
    return <Text role="alert">{error}</Text>;
  }

  if (!config) {
    return <Text>Loading overlay settings…</Text>;
  }

  return <OverlayForm governorId={governorId} config={config} commanders={commanders} />;
}
