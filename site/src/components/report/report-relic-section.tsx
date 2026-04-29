"use client";

import { useExtracted } from "next-intl";
import { ReportRelicSlot } from "@/components/report/report-relic-slot";
import { Subheading } from "@/components/ui/heading";
import type { RawRelicInfo } from "@/lib/types/raw-report";

type ReportRelicSectionProps = {
  relics: RawRelicInfo[];
};

export function ReportRelicSection({ relics }: ReportRelicSectionProps) {
  const t = useExtracted();
  const visibleRelics = relics.filter((relic) => isFiniteRelicId(relic.id));

  if (visibleRelics.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Subheading>{t("Relics")}</Subheading>
      <div className="flex flex-wrap gap-2">
        {visibleRelics.map((relic, index) => (
          <div key={`${relic.id}-${index}`}>
            <ReportRelicSlot relic={relic} />
          </div>
        ))}
      </div>
    </div>
  );
}

function isFiniteRelicId(id: unknown): id is number {
  return typeof id === "number" && Number.isFinite(id) && id > 0;
}
