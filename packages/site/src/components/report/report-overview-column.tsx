"use client";

import { useExtracted } from "next-intl";
import { Fragment, type ReactNode } from "react";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/description-list";
import { Subheading } from "@/components/ui/heading";
import { GameTranslate } from "@/components/v1/game-translate";
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
  const t = useExtracted();
  const participantName = participant?.player_name?.trim();
  const sideTitle =
    side === "self" ? (
      participantName || t("Unknown")
    ) : (
      <GameTranslate value="LC_COMMON_BATTLEREPORT_ALLENEMY" />
    );
  return (
    <div className="space-y-3 rounded bg-zinc-600/10 p-4 dark:bg-white/5">
      <Subheading>{sideTitle}</Subheading>
      <DescriptionList>
        {OVERVIEW_METRICS.map((metric) => {
          const key = side === "self" ? metric.selfKey : metric.enemyKey;
          const value = getOverviewValue(overview, key);
          let label: ReactNode;
          switch (metric.labelKey) {
            case "troopUnits":
              label = t("Troop Units");
              break;
            case "dead":
              label = <GameTranslate value="LC_COMMON_DEATH" />;
              break;
            case "severelyWounded":
              label = <GameTranslate value="LC_COMMON_SEVERELY_WOUNDED" />;
              break;
            case "slightlyWounded":
              label = <GameTranslate value="LC_COMMON_SLIGHTLY_WOUNDED" />;
              break;
            case "remaining":
              label = <GameTranslate value="LC_COMMON_UNITS_REMAINING" />;
              break;
            case "killPoints":
              label = <GameTranslate value="LC_COMMON_KILL_SCORE" />;
              break;
            default:
              return null;
          }
          return (
            <Fragment key={`${side}-${metric.labelKey}`}>
              <DescriptionTerm className="pt-1! pb-1! border-none!">{label}</DescriptionTerm>
              <DescriptionDetails className="pb-1! pt-1! border-none! sm:text-right tabular-nums">
                {value == null ? t("N/A") : formatter.format(value)}
              </DescriptionDetails>
            </Fragment>
          );
        })}
      </DescriptionList>
    </div>
  );
}
