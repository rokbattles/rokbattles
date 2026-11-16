"use client";

import { type JSX, use } from "react";
import { GovernorOverlaySettings } from "@/components/(app)/account/settings/stream-overlay/governor-overlay-settings";
import { Text } from "@/components/ui/text";
import type { OverlayCommander } from "@/lib/(app)/types/overlay";
import { GovernorContext } from "@/providers/governor-context";

type OverlaySettingsProps = {
  commanders: Record<string, OverlayCommander>;
};

export function OverlaySettings({ commanders }: OverlaySettingsProps): JSX.Element {
  const governor = use(GovernorContext)?.activeGovernor;

  if (!governor) {
    return <Text>Select a governor to configure your stream overlay.</Text>;
  }

  return (
    <GovernorOverlaySettings
      key={governor.governorId}
      governorId={governor.governorId}
      commanders={commanders}
    />
  );
}
