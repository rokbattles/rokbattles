import type { JSX } from "react";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { BattleCards } from "@/components/(app)/overlay/battle-cards";
import { overlaySampleBattles } from "@/lib/(app)/overlay-samples";
import type { OverlayCommander, OverlayPreferences } from "@/lib/(app)/types/overlay";

type OverlayPreviewProps = {
  settings: OverlayPreferences;
  commanders: Record<string, OverlayCommander>;
};

export function OverlayPreview({ settings, commanders }: OverlayPreviewProps): JSX.Element {
  return (
    <div role="region" aria-label="Overlay preview" className="min-w-0 xl:sticky xl:top-6">
      <SettingsSection title="Preview">
        <div className="@container">
          <div className="[zoom:0.6] @min-[260px]:[zoom:0.65] @min-[300px]:[zoom:0.75] @min-[350px]:[zoom:0.85] @min-[390px]:[zoom:1] [&_main]:max-w-none">
            <BattleCards
              battles={overlaySampleBattles.slice(0, settings.limit)}
              commanders={commanders}
              branding={settings.branding}
              status="test"
            />
          </div>
        </div>
      </SettingsSection>
    </div>
  );
}
