"use client";

import { ReportGearDiamond } from "@/components/report/report-gear-diamond";
import { Subheading } from "@/components/ui/heading";
import type { EquipmentToken } from "@/lib/report/parsers";

type ReportEquipmentSectionProps = {
  tokens: EquipmentToken[];
};

export function ReportEquipmentSection({ tokens }: ReportEquipmentSectionProps) {
  return (
    <div className="space-y-2">
      <Subheading>Equipment</Subheading>
      <div className="flex justify-center">
        <ReportGearDiamond tokens={tokens} />
      </div>
    </div>
  );
}
