"use client";

import { Fragment } from "react";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/description-list";
import { Subheading } from "@/components/ui/heading";
import { getOverviewValue, OVERVIEW_METRICS } from "@/lib/report/overview-metrics";
import type { RawOverview, RawParticipantInfo } from "@/lib/types/raw-report";

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
  const sideTitle = side === "self" ? participantName || "Unknown" : "All Enemies";
  return (
    <div className="space-y-3 rounded p-4 bg-white/5">
      <Subheading>{sideTitle}</Subheading>
      <DescriptionList>
        {OVERVIEW_METRICS.map((metric) => {
          const key = side === "self" ? metric.selfKey : metric.enemyKey;
          const value = getOverviewValue(overview, key);
          let label: string;
          switch (metric.labelKey) {
            case "troopUnits":
              label = "Troop Units";
              break;
            case "dead":
              label = "Dead";
              break;
            case "severelyWounded":
              label = "Severely Wounded";
              break;
            case "slightlyWounded":
              label = "Slightly Wounded";
              break;
            case "remaining":
              label = "Remaining";
              break;
            case "killPoints":
              label = "Kill Points";
              break;
            default:
              return null;
          }
          return (
            <Fragment key={`${side}-${metric.labelKey}`}>
              <DescriptionTerm className="pt-1! pb-1! border-none!">{label}</DescriptionTerm>
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
