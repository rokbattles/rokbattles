"use client";

import { useTranslations } from "next-intl";
import { ReportOverviewColumn } from "@/components/report/report-overview-column";
import { Subheading } from "@/components/ui/heading";
import type { RawOverview, RawParticipantInfo } from "@/lib/types/raw-report";

export function ReportOverviewCard({
  overview,
  selfParticipant,
  enemyParticipant,
}: {
  overview: RawOverview;
  selfParticipant?: RawParticipantInfo;
  enemyParticipant?: RawParticipantInfo;
}) {
  const t = useTranslations("report");
  const formatter = new Intl.NumberFormat("en-US", {
    maximumFractionDigits: 0,
  });

  return (
    <div className="space-y-4">
      <Subheading>{t("overview.title")}</Subheading>
      <div className="grid gap-4 sm:gap-6 md:grid-cols-2">
        <ReportOverviewColumn
          formatter={formatter}
          overview={overview}
          participant={selfParticipant}
          side="self"
        />
        <ReportOverviewColumn
          formatter={formatter}
          overview={overview}
          participant={enemyParticipant}
          side="enemy"
        />
      </div>
    </div>
  );
}
