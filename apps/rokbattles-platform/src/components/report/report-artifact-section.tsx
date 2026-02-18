"use client";

import { useExtracted } from "next-intl";
import { ReportEquipmentSlot } from "@/components/report/report-equipment-slot";
import { Subheading } from "@/components/ui/heading";
import type { EquipmentToken } from "@/lib/report/parsers";

type ReportArtifactSectionProps = {
  tokens: EquipmentToken[];
};

export function ReportArtifactSection({ tokens }: ReportArtifactSectionProps) {
  const t = useExtracted();
  if (tokens.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Subheading>{t("Artifacts")}</Subheading>
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
