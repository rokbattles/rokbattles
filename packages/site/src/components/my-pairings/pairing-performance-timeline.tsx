"use client";

import {
  CartesianGrid,
  Line,
  LineChart,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { PairingPerformanceResponse } from "@/lib/pairings";
import {
  formatPerformanceAxis,
  formatPerformanceValue,
  type PerformanceChart,
} from "@/lib/pairings/performance";

const shortDate = new Intl.DateTimeFormat("en-US", {
  month: "short",
  day: "numeric",
  timeZone: "UTC",
});
const fullDate = new Intl.DateTimeFormat("en-US", {
  month: "long",
  day: "numeric",
  year: "numeric",
  timeZone: "UTC",
});
const dateValue = (value: string) => new Date(`${value}T00:00:00Z`);

export function PerformanceTimeline({
  chart,
  series,
}: {
  chart: PerformanceChart;
  series: PairingPerformanceResponse["series"];
}) {
  const points = series.map((point) => ({ date: point.date, ...point.metrics }));
  const connectGaps = chart.id === "trade" || chart.format === "rate";
  const singlePoint = connectGaps
    ? points.filter((point) => point[chart.series[0].key] != null).length === 1
    : points.length === 1;

  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        accessibilityLayer
        data={points}
        margin={{ top: 8, right: 12, bottom: 4, left: 0 }}
      >
        <CartesianGrid vertical={false} stroke="#ffffff" strokeOpacity={0.06} />
        <XAxis
          dataKey="date"
          tickFormatter={(value) => shortDate.format(dateValue(value))}
          axisLine={false}
          tickLine={false}
          tick={{ fontSize: 11, fill: "#a1a1aa" }}
          tickMargin={12}
          minTickGap={40}
        />
        <YAxis
          tickFormatter={(value) => formatPerformanceAxis(Number(value), chart.format)}
          domain={[0, "auto"]}
          allowDecimals={chart.format !== "number"}
          axisLine={false}
          tickLine={false}
          tick={{ fontSize: 11, fill: "#a1a1aa" }}
          width={58}
        />
        <Tooltip
          content={<PerformanceTooltip chart={chart} />}
          cursor={{ stroke: "#71717a", strokeDasharray: "3 3" }}
        />
        {chart.id === "trade" ? (
          <ReferenceLine y={100} stroke="#71717a" strokeDasharray="4 4" ifOverflow="extendDomain" />
        ) : null}
        {chart.series.map((series) => (
          <Line
            key={series.key}
            dataKey={series.key}
            name={series.label}
            stroke={series.color}
            strokeWidth={2}
            type="linear"
            dot={singlePoint ? { r: 2.5, strokeWidth: 0 } : false}
            activeDot={{ r: 4, strokeWidth: 0 }}
            isAnimationActive={false}
            connectNulls={connectGaps}
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  );
}

function PerformanceTooltip({
  active,
  label,
  payload,
  chart,
}: {
  active?: boolean;
  label?: string | number;
  payload?: { dataKey?: string | number; value?: number | null }[];
  chart: PerformanceChart;
}) {
  if (!active || !payload?.length || typeof label !== "string") return null;

  return (
    <div className="rounded-sm border border-white/10 bg-zinc-900 px-3 py-2 text-xs shadow-sm">
      <p className="mb-2 text-zinc-400">{fullDate.format(dateValue(label))} · UTC</p>
      {chart.series.map((series) => {
        const value = payload.find((point) => point.dataKey === series.key)?.value ?? null;
        return (
          <div key={series.key} className="flex items-center justify-between gap-5 py-0.5">
            <span style={{ color: series.color }}>{series.label}</span>
            <span className="font-medium text-white tabular-nums">
              {formatPerformanceValue(value, chart.format, true)}
            </span>
          </div>
        );
      })}
    </div>
  );
}
