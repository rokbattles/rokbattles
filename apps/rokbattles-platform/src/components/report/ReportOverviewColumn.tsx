import { Fragment } from "react";
import { Strong } from "@/components/ui/Text";
import type { RawOverview, RawParticipantInfo } from "@/lib/types/rawReport";
import { getOverviewValue, OVERVIEW_METRICS } from "@/lib/report/overviewMetrics";

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
    <div className="space-y-4 rounded-xl bg-zinc-50/60 p-4 ring-1 ring-zinc-950/5 dark:bg-white/5 dark:ring-white/10">
      <Strong className="block text-sm font-semibold text-zinc-900 dark:text-white">
        {side === "self" ? participantName || "Unknown" : "All Enemies"}
      </Strong>
      <dl className="grid grid-cols-[1fr_auto] gap-x-4 gap-y-2 text-sm">
        {OVERVIEW_METRICS.map((metric) => {
          const key = side === "self" ? metric.selfKey : metric.enemyKey;
          const value = getOverviewValue(overview, key);
          return (
            <Fragment key={`${side}-${metric.label}`}>
              <dt className="text-zinc-500 dark:text-zinc-400">{metric.label}</dt>
              <dd className="text-right font-mono text-zinc-900 dark:text-white">
                {value == null ? "N/A" : formatter.format(value)}
              </dd>
            </Fragment>
          );
        })}
      </dl>
    </div>
  );
}
