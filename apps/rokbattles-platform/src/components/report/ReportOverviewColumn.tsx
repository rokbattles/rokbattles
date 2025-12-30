import { Fragment } from "react";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/DescriptionList";
import { Subheading } from "@/components/ui/Heading";
import { getOverviewValue, OVERVIEW_METRICS } from "@/lib/report/overviewMetrics";
import type { RawOverview, RawParticipantInfo } from "@/lib/types/rawReport";

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
  const participantName = participant?.player_name?.trim();
  return (
    <div className="space-y-3 rounded bg-zinc-600/10 p-4 dark:bg-white/5">
      <Subheading>{side === "self" ? participantName || "Unknown" : "All Enemies"}</Subheading>
      <DescriptionList>
        {OVERVIEW_METRICS.map((metric) => {
          const key = side === "self" ? metric.selfKey : metric.enemyKey;
          const value = getOverviewValue(overview, key);
          return (
            <Fragment key={`${side}-${metric.label}`}>
              <DescriptionTerm className="pt-1! pb-1! border-none!">{metric.label}</DescriptionTerm>
              <DescriptionDetails className="pb-1! pt-1! border-none! sm:text-right tabular-nums">
                {value == null ? "N/A" : formatter.format(value)}
              </DescriptionDetails>
            </Fragment>
          );
        })}
      </DescriptionList>
    </div>
  );
}
