"use client";

import dynamic from "next/dynamic";
import { useId } from "react";
import {
  PerformanceTimelineSkeleton,
  PerformanceValueSkeleton,
} from "@/components/my-pairings/pairing-performance-skeleton";
import { Subheading } from "@/components/ui/heading";
import type { PairingPerformanceResponse } from "@/lib/pairings";
import { durationPoints, formatBattleLength } from "@/lib/pairings/duration";
import { formatBattleDuration } from "@/lib/pairings/performance";

const DurationTimePlot = dynamic(
  () => import("./pairing-duration-plots").then((module) => module.DurationTimePlot),
  { loading: PerformanceTimelineSkeleton }
);
const DurationSpreadPlot = dynamic(
  () => import("./pairing-duration-plots").then((module) => module.DurationSpreadPlot),
  { loading: PerformanceTimelineSkeleton }
);

export function PairingDuration({
  data,
  loading,
}: {
  data: PairingPerformanceResponse | null;
  loading: boolean;
}) {
  const headingId = useId();
  const syncId = useId();

  const distribution = data?.totals.durationDistribution;
  const points = data ? durationPoints(data.series) : [];
  const timed = distribution?.count ?? 0;

  return (
    <section className="space-y-3" aria-labelledby={headingId}>
      <Subheading id={headingId}>Battle Duration</Subheading>
      <div
        className="min-w-0 overflow-hidden rounded-md border border-white/10 bg-white/5"
        role="group"
        aria-busy={loading}
        aria-label="Battle Duration"
      >
        {loading ? (
          <span className="sr-only" role="status">
            Loading battle durations…
          </span>
        ) : null}
        <dl className="grid grid-cols-2 border-b border-white/10">
          <div className="min-w-0 px-2 py-4 sm:px-4">
            <dt className="min-h-8 text-[11px] font-medium text-zinc-400 sm:min-h-5 sm:text-sm">
              Total Battle Time
            </dt>
            <dd className="mt-2 break-words text-sm font-semibold text-white tabular-nums sm:text-lg">
              {data ? (
                formatBattleDuration(data.totals.battleDurationMillis)
              ) : (
                <PerformanceValueSkeleton>000h 00m</PerformanceValueSkeleton>
              )}
            </dd>
          </div>
          <div className="min-w-0 px-2 py-4 sm:px-4">
            <dt className="min-h-8 text-[11px] font-medium text-zinc-400 sm:min-h-5 sm:text-sm">
              Median Battle Duration
            </dt>
            <dd className="mt-2 break-words text-sm font-semibold text-white tabular-nums sm:text-lg">
              {data ? (
                formatBattleLength(distribution?.p50Millis)
              ) : (
                <PerformanceValueSkeleton>000h 00m</PerformanceValueSkeleton>
              )}
            </dd>
          </div>
        </dl>
        <div className="space-y-5 p-3 sm:p-5">
          <div>
            <h3 className="mb-3 text-xs font-medium text-zinc-300">Battle Duration</h3>
            <div
              className="h-36 min-w-0 sm:h-40"
              role="group"
              aria-label="Total battle time by day"
            >
              {data ? (
                <DurationTimePlot points={points} syncId={syncId} />
              ) : (
                <PerformanceTimelineSkeleton />
              )}
            </div>
          </div>
          <div className="border-t border-white/10 pt-5">
            <div className="mb-3 flex flex-wrap items-center justify-between gap-3 text-xs">
              <h3 className="font-medium text-zinc-300">Battle Duration Spread</h3>
              <ul
                className="flex flex-wrap gap-x-4 gap-y-2 text-[11px] text-zinc-400"
                aria-label="Duration percentile legend"
              >
                <li className="inline-flex items-center gap-1.5">
                  <span className="h-0.5 w-3 bg-violet-300" aria-hidden="true" />
                  P50
                </li>
                <li className="inline-flex items-center gap-1.5">
                  <span className="h-2 w-3 bg-violet-400/35" aria-hidden="true" />
                  P25–P75
                </li>
                <li className="inline-flex items-center gap-1.5">
                  <span className="h-2 w-3 bg-violet-400/15" aria-hidden="true" />
                  P10–P90
                </li>
              </ul>
            </div>
            <div
              className="h-56 min-w-0 sm:h-64"
              role="group"
              aria-label="Daily battle duration percentiles"
            >
              {!data ? (
                <PerformanceTimelineSkeleton />
              ) : timed ? (
                <DurationSpreadPlot points={points} syncId={syncId} />
              ) : (
                <div className="flex h-full items-center justify-center text-sm text-zinc-400">
                  No recorded battle durations in this range.
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
