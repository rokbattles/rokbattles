"use client";

import { useMemo } from "react";
import {
  CartesianGrid,
  Line,
  LineChart,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  XAxis,
  YAxis,
} from "recharts";
import { ResourcesTimelineTooltipClient } from "@/components/account-resources/resources-timeline-tooltip-client";
import {
  ONE_DAY_MILLIS,
  parseDateInput,
  toDateKey,
  toDateLabel,
  todayUtcStartMillis,
} from "@/lib/loot/date";
import { getLootName } from "@/lib/loot-catalog";
import { RESOURCE_TYPE_IDS } from "@/lib/resources/catalog";
import type { ResourcesDailyAggregate } from "@/lib/types/resources";

type ChartPoint = {
  label: string;
  [key: string]: number | string | null;
  crystalsGain: number | null;
};

type ChartSeries = {
  key: string;
  color: string;
};

type ResourcesTimelineChartClientProps = {
  data: ResourcesDailyAggregate[];
  rangeStart: string;
  rangeEnd: string;
  datasetLocale?: string;
};

const wholeNumberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

const abbreviatedNumberFormatter = new Intl.NumberFormat("en-US", {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

const resourceTypeLineColors: Record<number, string> = {
  1: "#22c55e",
  2: "#92400e",
  3: "#6b7280",
  4: "#eab308",
  5: "#ef4444",
};

function formatResourcesAxisNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "0";
  }

  const rounded = Math.round(value);
  const absolute = Math.abs(rounded);

  if (absolute < 100_000) {
    return wholeNumberFormatter.format(rounded);
  }

  if (absolute >= 1_000_000) {
    return `${abbreviatedNumberFormatter.format(rounded / 1_000_000)}M`;
  }

  return `${abbreviatedNumberFormatter.format(rounded / 1_000)}K`;
}

export function ResourcesTimelineChartClient({
  data,
  rangeStart,
  rangeEnd,
  datasetLocale,
}: ResourcesTimelineChartClientProps) {
  const chartData = useMemo(
    () =>
      data
        .map((entry) => ({
          date: entry.date,
          crystalsGain: entry.crystalsGain,
          resources: entry.resources,
        }))
        .sort((a, b) => a.date.localeCompare(b.date)),
    [data]
  );

  const series = useMemo(() => {
    const resourceSeries = RESOURCE_TYPE_IDS.map<ChartSeries>((typeId) => ({
      key: `type:${typeId}`,
      color: resourceTypeLineColors[typeId],
    }));

    return [...resourceSeries, { key: "crystalsGain", color: "#38bdf8" }];
  }, []);

  const labels = useMemo(() => {
    const next: Record<string, string> = {};

    for (const typeId of RESOURCE_TYPE_IDS) {
      const resourceName = getLootName(1, typeId, datasetLocale);
      if (!resourceName) {
        throw new Error(`Missing resource subtype ${typeId} in loot dataset type 1`);
      }

      next[`type:${typeId}`] = resourceName;
    }

    const crystalsName = getLootName(1, 9, datasetLocale);
    if (!crystalsName) {
      throw new Error("Missing crystals name in loot dataset");
    }
    next.crystalsGain = crystalsName;

    return next;
  }, [datasetLocale]);

  const displayData = useMemo(() => {
    if (chartData.length === 0) {
      return [] satisfies ChartPoint[];
    }

    const firstDataDate = parseDateInput(chartData[0]?.date ?? "");
    const lastDataDate = parseDateInput(chartData[chartData.length - 1]?.date ?? "");
    const rangeStartMillis = parseDateInput(rangeStart) ?? firstDataDate;
    const rangeEndMillis = parseDateInput(rangeEnd) ?? lastDataDate;

    if (rangeStartMillis == null || rangeEndMillis == null || rangeEndMillis < rangeStartMillis) {
      return [] satisfies ChartPoint[];
    }

    const resourcesMap = new Map(chartData.map((entry) => [entry.date, entry]));
    const todayMillis = todayUtcStartMillis();

    const points: ChartPoint[] = [];
    for (let cursor = rangeStartMillis; cursor <= rangeEndMillis; cursor += ONE_DAY_MILLIS) {
      const dateKey = toDateKey(cursor);
      const isFuture = cursor > todayMillis;
      const day = resourcesMap.get(dateKey);
      const point: ChartPoint = {
        label: toDateLabel(dateKey),
        crystalsGain: isFuture ? null : (day?.crystalsGain ?? 0),
      };

      const dayResources = new Map<string, number>(
        (day?.resources ?? []).map((resource) => [`type:${resource.type}`, resource.total])
      );

      for (const item of series) {
        if (item.key === "crystalsGain") {
          point[item.key] = isFuture ? null : (day?.crystalsGain ?? 0);
          continue;
        }

        point[item.key] = isFuture ? null : (dayResources.get(item.key) ?? 0);
      }

      points.push(point);
    }

    return points;
  }, [chartData, rangeEnd, rangeStart, series]);

  const axisTicks = useMemo(() => {
    if (displayData.length === 0) {
      return [] as string[];
    }

    const firstLabel = displayData[0]?.label ?? "";
    const lastLabel = displayData[displayData.length - 1]?.label ?? firstLabel;
    if (firstLabel === lastLabel) {
      return [firstLabel];
    }

    return [firstLabel, lastLabel];
  }, [displayData]);

  if (displayData.length === 0) {
    return null;
  }

  return (
    <div className="h-[300px] w-full">
      <ResponsiveContainer>
        <LineChart data={displayData} margin={{ top: 8, right: 24, bottom: 12, left: 12 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#3f3f46" vertical={false} />
          <XAxis
            dataKey="label"
            ticks={axisTicks}
            tick={{ fontSize: 12, fill: "#a1a1aa" }}
            tickLine={false}
            interval={0}
            padding={{ left: 8, right: 8 }}
            tickMargin={8}
          />
          <YAxis
            tickFormatter={(value) => formatResourcesAxisNumber(Number(value))}
            tick={{ fontSize: 12, fill: "#a1a1aa" }}
            axisLine={false}
            tickLine={false}
            allowDecimals={false}
          />
          <RechartsTooltip
            cursor={{ fill: "rgba(63, 63, 70, 0.2)" }}
            content={
              <ResourcesTimelineTooltipClient
                labels={labels}
                order={series.map((item) => item.key)}
              />
            }
          />
          {series.map((item) => (
            <Line
              key={item.key}
              type="monotone"
              dataKey={item.key}
              stroke={item.color}
              strokeWidth={2.5}
              dot={false}
              connectNulls={false}
              isAnimationActive={false}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
