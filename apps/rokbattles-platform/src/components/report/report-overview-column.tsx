"use client";

import { useTranslations } from "next-intl";
import { Fragment } from "react";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/DescriptionList";
import { Subheading } from "@/components/ui/Heading";
import { getOverviewValue, OVERVIEW_METRICS } from "@/lib/report/overviewMetrics";
import type { RawOverview, RawParticipantInfo } from "@/lib/types/rawReport";

const COMMON_METRIC_KEYS = new Set(["dead", "remaining", "killPoints"]);

type ReportOverviewColumnProps = {
  side: "self" | "enemy";
  overview: RawOverview;
  participant?: RawParticipantInfo;
  formatter: Intl.NumberFormat;
};

export function ReportOverviewColumn({
  side,
  overview,
  participant,
  formatter,
}: ReportOverviewColumnProps) {
  const t = useTranslations("report");
  const tCommon = useTranslations("common");
  const participantName = participant?.player_name?.trim();
  const sideTitle =
    side === "self" ? participantName || tCommon("labels.unknown") : t("overview.allEnemies");
  return (
    <div className="space-y-3 rounded bg-zinc-600/10 p-4 dark:bg-white/5">
      <Subheading>{sideTitle}</Subheading>
      <DescriptionList>
        {OVERVIEW_METRICS.map((metric) => {
          const key = side === "self" ? metric.selfKey : metric.enemyKey;
          const value = getOverviewValue(overview, key);
          const label = COMMON_METRIC_KEYS.has(metric.labelKey)
            ? tCommon(`metrics.${metric.labelKey}`)
            : t(`overview.metrics.${metric.labelKey}`);
          return (
            <Fragment key={`${side}-${metric.labelKey}`}>
              <DescriptionTerm className="pt-1! pb-1! border-none!">{label}</DescriptionTerm>
              <DescriptionDetails className="pb-1! pt-1! border-none! sm:text-right tabular-nums">
                {value == null ? tCommon("labels.na") : formatter.format(value)}
              </DescriptionDetails>
            </Fragment>
          );
        })}
      </DescriptionList>
    </div>
  );
}
