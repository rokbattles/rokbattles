"use client";

import { cn } from "cn";
import { useExtracted, useFormatter } from "next-intl";
import { type ReactElement, useMemo } from "react";
import {
  Bar,
  CartesianGrid,
  ComposedChart,
  Line,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  XAxis,
  YAxis,
} from "recharts";
import { ResourcesTimelineTooltipClient } from "@/components/account-resources/resources-timeline-tooltip-client";
import { Subheading } from "@/components/ui/heading";
import {
  buildResourceChartPoints,
  type ResourceChartPoint,
  type ResourceDay,
} from "@/lib/resources/analytics";
import type { ResourceBreakdownRow } from "@/lib/resources/rows";

type ResourcesTimelineChartClientProps = {
  days: ResourceDay[];
  rows: ResourceBreakdownRow[];
  cumulative: boolean;
};

type ResourceChartProps = {
  points: ResourceChartPoint[];
  rows: ResourceBreakdownRow[];
  title: string;
  cumulative: boolean;
};

export function ResourcesTimelineChartClient({
  days,
  rows,
  cumulative,
}: ResourcesTimelineChartClientProps): ReactElement {
  const t = useExtracted();
  const standard = rows.filter((row) => row.key !== "type:5" && row.key !== "crystalsGain");
  const gems = rows.filter((row) => row.key === "type:5");
  const crystals = rows.filter((row) => row.key === "crystalsGain");
  const points = useMemo(() => buildResourceChartPoints(days, cumulative), [days, cumulative]);
  const common = { points, cumulative };

  return (
    <div className={cn("grid gap-4", gems.length > 0 && crystals.length > 0 && "lg:grid-cols-2")}>
      {standard.length > 0 ? (
        <div className="col-span-full">
          <ResourceChart {...common} rows={standard} title={t("Gathered resources")} />
        </div>
      ) : null}
      {gems.length > 0 ? <ResourceChart {...common} rows={gems} title={gems[0].name} /> : null}
      {crystals.length > 0 ? (
        <ResourceChart {...common} rows={crystals} title={crystals[0].name} />
      ) : null}
    </div>
  );
}

function ResourceChart({ points, rows, title, cumulative }: ResourceChartProps): ReactElement {
  const intl = useFormatter();

  return (
    <section className="min-w-0 rounded-md border border-zinc-200 p-4 sm:p-5 dark:border-zinc-800">
      <Subheading>{title}</Subheading>
      <div className="mt-4 h-64 w-full" role="group" aria-label={title}>
        <ResponsiveContainer>
          <ComposedChart
            accessibilityLayer
            data={points}
            margin={{ top: 8, right: 8, left: 0, bottom: 4 }}
          >
            <CartesianGrid
              strokeDasharray="3 3"
              stroke="currentColor"
              className="text-zinc-200 dark:text-zinc-800"
              vertical={false}
            />
            <XAxis
              dataKey="date"
              tickFormatter={(value) =>
                intl.dateTime(new Date(`${value}T00:00:00Z`), {
                  month: "short",
                  day: "numeric",
                  timeZone: "UTC",
                })
              }
              tick={{ fontSize: 11, fill: "#71717a" }}
              axisLine={false}
              tickLine={false}
              minTickGap={36}
              tickMargin={12}
            />
            <YAxis
              tickFormatter={(value) =>
                intl.number(Number(value), { notation: "compact", maximumFractionDigits: 1 })
              }
              tick={{ fontSize: 11, fill: "#71717a" }}
              axisLine={false}
              tickLine={false}
              allowDecimals={false}
              width={58}
              domain={[0, "auto"]}
            />
            <RechartsTooltip content={<ResourcesTimelineTooltipClient rows={rows} />} />
            {rows.map((row) =>
              cumulative ? (
                <Line
                  key={row.key}
                  type="linear"
                  dataKey={row.key}
                  name={row.name}
                  stroke={row.color}
                  strokeWidth={2}
                  dot={points.length === 1}
                  activeDot={{ r: 4 }}
                  isAnimationActive={false}
                />
              ) : (
                <Bar
                  key={row.key}
                  dataKey={row.key}
                  name={row.name}
                  fill={row.color}
                  stackId="gathered"
                  maxBarSize={48}
                  isAnimationActive={false}
                />
              )
            )}
          </ComposedChart>
        </ResponsiveContainer>
      </div>
    </section>
  );
}
