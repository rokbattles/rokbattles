"use client";

import { Tabs } from "@base-ui/react/tabs";
import dynamic from "next/dynamic";
import { useId } from "react";
import { PairingDuration } from "@/components/my-pairings/pairing-duration";
import {
  PerformanceTimelineSkeleton,
  PerformanceValueSkeleton,
} from "@/components/my-pairings/pairing-performance-skeleton";
import { PairingsState } from "@/components/my-pairings/pairings-shared";
import { Subheading } from "@/components/ui/heading";
import { usePairingsQuery } from "@/hooks/use-pairings";
import type { PairingPerformanceResponse } from "@/lib/pairings";
import {
  formatPerformanceValue,
  PERFORMANCE_CHARTS,
  type PerformanceChart,
  TEMPO_CHARTS,
} from "@/lib/pairings/performance";

const PerformanceTimeline = dynamic(
  () => import("./pairing-performance-timeline").then((module) => module.PerformanceTimeline),
  {
    loading: PerformanceTimelineSkeleton,
  }
);

export function PairingPerformance({ url }: { url: string | null }) {
  const result = usePairingsQuery<PairingPerformanceResponse>(url);

  if (result.error) return <PairingsState error={result.error} retry={result.retry} />;
  if (!result.loading && !result.data?.totals.battles)
    return <PairingsState>No battles for this loadout in this range.</PairingsState>;
  const data = result.data;

  return (
    <div className="space-y-8">
      <MetricChartGroup
        title="Battle Performance"
        charts={PERFORMANCE_CHARTS}
        data={data}
        loading={result.loading}
      />
      <MetricChartGroup
        title="Battle Tempo"
        charts={TEMPO_CHARTS}
        data={data}
        loading={result.loading}
      />
      <PairingDuration data={data} loading={result.loading} />
    </div>
  );
}

function MetricChartGroup({
  title,
  charts,
  data,
  loading,
}: {
  title: string;
  charts: PerformanceChart[];
  data: PairingPerformanceResponse | null;
  loading: boolean;
}) {
  const headingId = useId();

  return (
    <section className="space-y-3" aria-labelledby={headingId}>
      <Subheading id={headingId}>{title}</Subheading>
      <Tabs.Root
        defaultValue={charts[0].id}
        className="min-w-0 overflow-hidden rounded-md border border-white/10"
        aria-label={title}
        aria-busy={loading}
      >
        {loading ? (
          <span className="sr-only" role="status">
            Loading {title}…
          </span>
        ) : null}
        <Tabs.List
          aria-label={`${title} metrics`}
          className="grid w-full items-stretch"
          style={{ gridTemplateColumns: `repeat(${charts.length}, minmax(0, 1fr))` }}
        >
          {charts.map((chart) => (
            <Tabs.Tab
              key={chart.id}
              value={chart.id}
              className="group flex min-w-0 flex-col items-stretch rounded-t-sm px-2 py-4 text-left text-zinc-400 transition-colors hover:text-white focus-visible:z-10 focus-visible:outline-2 focus-visible:outline-inset focus-visible:outline-blue-500 data-active:bg-white/5 data-active:text-white sm:px-4"
            >
              <span className="min-h-8 text-[11px] font-medium sm:min-h-5 sm:text-sm">
                {chart.label}
              </span>
              <span className="mt-2 flex flex-col gap-1">
                {chart.series.map((series) => (
                  <span
                    key={series.key}
                    className="flex min-w-0 flex-col items-start sm:flex-row sm:flex-wrap sm:items-baseline sm:gap-x-2"
                    title={
                      data
                        ? `${series.label}: ${formatPerformanceValue(data.totals[series.key], chart.format, true)}`
                        : undefined
                    }
                  >
                    <span
                      className="break-words text-sm font-semibold tabular-nums sm:text-lg"
                      style={{ color: chart.series.length > 1 ? series.color : undefined }}
                    >
                      {!data ? (
                        <PerformanceValueSkeleton />
                      ) : chart.id === "trade" &&
                        data.totals.tradePercent === null &&
                        data.totals.kpGain > 0 ? (
                        "No KP lost"
                      ) : (
                        formatPerformanceValue(
                          data.totals[series.key],
                          chart.format,
                          series.key === "battles"
                        )
                      )}
                    </span>
                    {series.totalLabel ? (
                      <span className="text-[10px] font-normal text-zinc-500 sm:text-xs">
                        {series.totalLabel}
                      </span>
                    ) : null}
                  </span>
                ))}
              </span>
            </Tabs.Tab>
          ))}
        </Tabs.List>
        {charts.map((chart) => (
          <Tabs.Panel
            key={chart.id}
            value={chart.id}
            className="min-w-0 rounded-b-sm bg-white/5 p-3 outline-none sm:p-5"
          >
            <div className="mb-5 flex flex-wrap items-center justify-end gap-3 text-xs text-zinc-400">
              <ul className="flex flex-wrap gap-x-5 gap-y-2" aria-label="Chart legend">
                {chart.series.map((series) => (
                  <li key={series.key} className="inline-flex items-center gap-2">
                    <span
                      className="h-0.5 w-4"
                      style={{ backgroundColor: series.color }}
                      aria-hidden="true"
                    />
                    {series.label}
                  </li>
                ))}
              </ul>
            </div>
            <div
              className="h-72 min-w-0 sm:h-80"
              role="group"
              aria-label={`${chart.label} by day in UTC`}
            >
              {data ? (
                <PerformanceTimeline chart={chart} series={data.series} />
              ) : (
                <PerformanceTimelineSkeleton />
              )}
            </div>
          </Tabs.Panel>
        ))}
      </Tabs.Root>
    </section>
  );
}
