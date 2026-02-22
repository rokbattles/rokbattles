"use client";

import { useExtracted } from "next-intl";
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
import { LootTimelineTooltipClient } from "@/components/account-loot/loot-timeline-tooltip-client";
import {
  ONE_DAY_MILLIS,
  parseDateInput,
  toDateKey,
  toDateLabel,
  todayUtcStartMillis,
} from "@/lib/loot/date";
import type { LootDailyAggregate } from "@/lib/types/loot";

type ChartPoint = {
  label: string;
  value: number | null;
};

const numberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

type LootTimelineChartClientProps = {
  data: LootDailyAggregate[];
  rangeStart: string;
  rangeEnd: string;
};

export function LootTimelineChartClient({
  data,
  rangeStart,
  rangeEnd,
}: LootTimelineChartClientProps) {
  const t = useExtracted();
  const chartData = useMemo(
    () =>
      data
        .map((entry) => ({ date: entry.date, reports: entry.reports }))
        .sort((a, b) => a.date.localeCompare(b.date)),
    [data]
  );

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

    const reportMap = new Map(chartData.map((entry) => [entry.date, entry.reports]));
    const todayMillis = todayUtcStartMillis();

    const points: ChartPoint[] = [];
    for (let cursor = rangeStartMillis; cursor <= rangeEndMillis; cursor += ONE_DAY_MILLIS) {
      const dateKey = toDateKey(cursor);
      const isFuture = cursor > todayMillis;
      points.push({
        label: toDateLabel(dateKey),
        value: isFuture ? null : (reportMap.get(dateKey) ?? 0),
      });
    }

    return points;
  }, [chartData, rangeEnd, rangeStart]);

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
            tickFormatter={(value) => numberFormatter.format(value)}
            tick={{ fontSize: 12, fill: "#a1a1aa" }}
            axisLine={false}
            tickLine={false}
            allowDecimals={false}
          />
          <RechartsTooltip
            cursor={{ fill: "rgba(63, 63, 70, 0.2)" }}
            content={<LootTimelineTooltipClient killsLabel={t("Kills")} />}
          />
          <Line
            type="monotone"
            dataKey="value"
            stroke="#3b82f6"
            strokeWidth={2.5}
            dot={false}
            connectNulls={false}
            isAnimationActive={false}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
