"use client";

import { EquipmentDiamond } from "@/components/equipment-diamond";
import { Subheading } from "@/components/ui/heading";
import type { EquipmentToken } from "@/lib/report/parsers";

type ReportEquipmentSectionProps = {
  tokens: EquipmentToken[];
  commanderId?: number;
};

export function ReportEquipmentSection({ tokens, commanderId }: ReportEquipmentSectionProps) {
  return (
    <div className="space-y-2">
      <Subheading>Equipment</Subheading>
      <EquipmentDiamond tokens={tokens} commanderId={commanderId} />
    </div>
  );
}
