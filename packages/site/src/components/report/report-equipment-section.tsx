"use client";

import { ReportGearDiamond } from "@/components/report/report-gear-diamond";
import { Subheading } from "@/components/ui/heading";
import { getCommanderTalents } from "@/lib/commander";
import type { EquipmentToken } from "@/lib/report/parsers";

type ReportEquipmentSectionProps = {
  tokens: EquipmentToken[];
  commanderId?: number;
};

export function ReportEquipmentSection({ tokens, commanderId }: ReportEquipmentSectionProps) {
  const commanderTalents = getCommanderTalents(commanderId);

  return (
    <div className="space-y-2">
      <Subheading>Equipment</Subheading>
      <div className="flex justify-center">
        <ReportGearDiamond tokens={tokens} commanderTalents={commanderTalents} />
      </div>
    </div>
  );
}
