import { Fragment, useMemo } from "react";
import { Subheading } from "@/components/ui/Heading";
import { Strong } from "@/components/ui/Text";
import type { RawOverview, RawParticipantInfo } from "@/lib/types/rawReport";

export const OVERVIEW_METRICS = [
  { label: "Troop Units", selfKey: "max", enemyKey: "enemy_max" },
  { label: "Dead", selfKey: "death", enemyKey: "enemy_death" },
  {
    label: "Severely Wounded",
    selfKey: "severely_wounded",
    enemyKey: "enemy_severely_wounded",
  },
  { label: "Slightly Wounded", selfKey: "wounded", enemyKey: "enemy_wounded" },
  { label: "Remaining", selfKey: "remaining", enemyKey: "enemy_remaining" },
  { label: "Kill Points", selfKey: "kill_score", enemyKey: "enemy_kill_score" },
] as const satisfies readonly {
  label: string;
  selfKey: keyof RawOverview;
  enemyKey: keyof RawOverview;
}[];

export function ReportOverviewCard({
  overview,
  selfParticipant,
  enemyParticipant,
}: {
  overview: RawOverview;
  selfParticipant?: RawParticipantInfo;
  enemyParticipant?: RawParticipantInfo;
}) {
  const formatter = useMemo(
    () =>
      new Intl.NumberFormat("en-US", {
        maximumFractionDigits: 0,
      }),
    []
  );

  return (
    <div className="space-y-4">
      <Subheading>Data summary</Subheading>
      <div className="grid gap-4 sm:gap-6 md:grid-cols-2">
        <OverviewColumn
          side="self"
          overview={overview}
          participant={selfParticipant}
          formatter={formatter}
        />
        <OverviewColumn
          side="enemy"
          overview={overview}
          participant={enemyParticipant}
          formatter={formatter}
        />
      </div>
    </div>
  );
}

function OverviewColumn({
  side,
  overview,
  participant,
  formatter,
}: {
  side: "self" | "enemy";
  overview: RawOverview;
  participant?: RawParticipantInfo;
  formatter: Intl.NumberFormat;
}) {
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

function getOverviewValue(overview: RawOverview, key: keyof RawOverview) {
  const value = overview?.[key];
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return null;
  }
  return value;
}

export function hasOverviewData(overview: RawOverview) {
  return OVERVIEW_METRICS.some((metric) => {
    const selfValue = getOverviewValue(overview, metric.selfKey);
    const enemyValue = getOverviewValue(overview, metric.enemyKey);
    return selfValue != null || enemyValue != null;
  });
}
