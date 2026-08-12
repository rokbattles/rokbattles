"use client";

import { ReportEquipmentSlot } from "@/components/report/report-equipment-slot";
import { Subheading } from "@/components/ui/heading";
import { GameTranslate } from "@/components/v1/game-translate";
import type { EquipmentToken } from "@/lib/report/parsers";

type ReportArtifactSectionProps = {
  tokens: EquipmentToken[];
};

export function ReportArtifactSection({ tokens }: ReportArtifactSectionProps) {
  if (tokens.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Subheading>
        <GameTranslate value="LC_KINGDOMWAR_S14_TROOPSEQUIP_BTN" />
      </Subheading>
      <div className="grid grid-cols-4 gap-3 sm:grid-cols-5">
        {tokens.map((token) => (
          <div key={`${token.slot}-${token.id}`}>
            <ReportEquipmentSlot token={token} />
          </div>
        ))}
      </div>
    </div>
  );
}
