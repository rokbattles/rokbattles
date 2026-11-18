"use client";

import {
  Area,
  Bar,
  BarChart,
  CartesianGrid,
  ComposedChart,
  Line,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { formatExact } from "@/lib/pairings/analytics";
import {
  DURATION_PERCENTILES,
  type DurationPoint,
  formatBattleLength,
} from "@/lib/pairings/duration";
import { formatBattleDuration, formatPerformanceAxis } from "@/lib/pairings/performance";

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
const ticks = { fontSize: 11, fill: "#a1a1aa" };
const margin = { top: 8, right: 12, bottom: 4, left: 0 };
const durationAxis = (value: number) => formatPerformanceAxis(Number(value), "duration");

type PlotProps = { points: DurationPoint[]; syncId: string };

export function DurationTimePlot({ points, syncId }: PlotProps) {
  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart accessibilityLayer data={points} syncId={syncId} syncMethod="value" margin={margin}>
        <CartesianGrid vertical={false} stroke="#ffffff" strokeOpacity={0.06} />
        <XAxis dataKey="date" scale="band" hide />
        <YAxis
          domain={[0, "auto"]}
          tickFormatter={durationAxis}
          axisLine={false}
          tickLine={false}
          tick={ticks}
          width={58}
        />
        <Tooltip
          content={<DurationTooltip view="total" />}
          cursor={{ fill: "#ffffff", fillOpacity: 0.035 }}
        />
        <Bar
          dataKey="totalMillis"
          name="Battle time"
          fill="#60a5fa"
          fillOpacity={0.65}
          maxBarSize={24}
          radius={[2, 2, 0, 0]}
          isAnimationActive={false}
        />
      </BarChart>
    </ResponsiveContainer>
  );
}

export function DurationSpreadPlot({ points, syncId }: PlotProps) {
  const singlePoint = points.filter((point) => point.distribution !== null).length === 1;

  return (
    <ResponsiveContainer width="100%" height="100%">
      <ComposedChart
        accessibilityLayer
        data={points}
        syncId={syncId}
        syncMethod="value"
        margin={margin}
      >
        <CartesianGrid vertical={false} stroke="#ffffff" strokeOpacity={0.06} />
        <XAxis
          dataKey="date"
          scale="band"
          tickFormatter={(value) => shortDate.format(dateValue(value))}
          minTickGap={40}
          axisLine={false}
          tickLine={false}
          tick={ticks}
          tickMargin={12}
        />
        <YAxis
          domain={[0, "auto"]}
          tickFormatter={durationAxis}
          axisLine={false}
          tickLine={false}
          tick={ticks}
          width={58}
        />
        <Tooltip
          content={<DurationTooltip view="spread" />}
          cursor={{ stroke: "#71717a", strokeDasharray: "3 3" }}
        />
        <Area
          dataKey="outerRange"
          name="P10–P90"
          type="linear"
          stroke="none"
          fill="#a78bfa"
          fillOpacity={0.12}
          connectNulls
          isAnimationActive={false}
        />
        <Area
          dataKey="innerRange"
          name="P25–P75"
          type="linear"
          stroke="none"
          fill="#a78bfa"
          fillOpacity={0.3}
          connectNulls
          isAnimationActive={false}
        />
        <Line
          dataKey="medianMillis"
          name="P50 · Median"
          type="linear"
          stroke="#c4b5fd"
          strokeWidth={2}
          dot={singlePoint ? { r: 3 } : false}
          activeDot={{ r: 4, strokeWidth: 0 }}
          connectNulls
          isAnimationActive={false}
        />
      </ComposedChart>
    </ResponsiveContainer>
  );
}

function DurationTooltip({
  active,
  label,
  payload,
  view,
}: {
  active?: boolean;
  label?: string | number;
  payload?: { payload?: DurationPoint }[];
  view: "total" | "spread";
}) {
  const point = payload?.[0]?.payload;
  if (!active || !point || typeof label !== "string") return null;

  const distribution = point.distribution;

  return (
    <div className="rounded-sm border border-white/10 bg-zinc-900 px-3 py-2 text-xs shadow-sm">
      <p className="mb-2 text-zinc-400">{fullDate.format(dateValue(label))} · UTC</p>
      {view === "total" ? (
        <>
          <div className="flex justify-between gap-5 py-0.5">
            <span className="text-zinc-400">Battle Duration</span>
            <span className="font-medium text-white tabular-nums">
              {formatBattleDuration(point.totalMillis)}
            </span>
          </div>
          <div className="flex justify-between gap-5 py-0.5">
            <span className="text-zinc-400">Battles</span>
            <span className="font-medium text-white tabular-nums">
              {formatExact(point.battles)}
            </span>
          </div>
        </>
      ) : (
        DURATION_PERCENTILES.map(([key, name]) => (
          <div key={key} className="flex justify-between gap-5 py-0.5">
            <span className={key === "p50Millis" ? "text-violet-300" : "text-zinc-400"}>
              {name}
            </span>
            <span className="font-medium text-white tabular-nums">
              {formatBattleLength(distribution?.[key])}
            </span>
          </div>
        ))
      )}
    </div>
  );
}
