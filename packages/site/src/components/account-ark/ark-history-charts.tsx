"use client";

import type { ReactElement } from "react";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { getArkTeams } from "@/lib/ark/analytics";
import type { ArkMatchRecord } from "@/lib/types/ark";

const dateFormatter = new Intl.DateTimeFormat("en-US", {
  month: "short",
  day: "numeric",
  timeZone: "UTC",
});
const compactNumberFormatter = new Intl.NumberFormat("en-US", { notation: "compact" });
const timeFormatter = new Intl.DateTimeFormat("en-US", {
  dateStyle: "medium",
  timeStyle: "short",
  timeZone: "UTC",
});
const numberFormatter = new Intl.NumberFormat("en-US");

type ArkHistoryChartsProps = { rows: ArkMatchRecord[] };

type ChartPoint = {
  time: number;
  own: number | null;
  opponent: number | null;
  personal: number | null;
};

type ArkHistoryChartProps = {
  title: string;
  points: ChartPoint[];
  series: { key: "own" | "opponent" | "personal"; name: string; color: string }[];
};

export function ArkHistoryCharts({ rows }: ArkHistoryChartsProps): ReactElement {
  const points = rows.toReversed().map((match) => {
    const { own, opponent } = getArkTeams(match);
    return {
      time: match.mailTimeMillis,
      own: own?.score ?? null,
      opponent: opponent?.score ?? null,
      personal: match.personalScore,
    };
  });

  return (
    <div className="grid gap-4 xl:grid-cols-2">
      <ArkHistoryChart
        title="Alliance score trend"
        points={points}
        series={[
          { key: "own", name: "Your alliance", color: "#3b82f6" },
          { key: "opponent", name: "Opponent", color: "#f43f5e" },
        ]}
      />
      <ArkHistoryChart
        title="Individual score trend"
        points={points}
        series={[{ key: "personal", name: "Your score", color: "#a78bfa" }]}
      />
    </div>
  );
}

function ArkHistoryChart({ title, points, series }: ArkHistoryChartProps): ReactElement {
  const hasValues = points.some((point) => series.some((entry) => point[entry.key] != null));

  return (
    <section className="min-w-0 rounded-md border p-4 sm:p-5 border-zinc-800">
      <Subheading>{title}</Subheading>
      {hasValues ? (
        <div className="mt-4 h-60" role="group" aria-label={title}>
          <ResponsiveContainer>
            <LineChart
              data={points}
              accessibilityLayer
              margin={{ top: 8, right: 12, left: 0, bottom: 4 }}
            >
              <CartesianGrid
                vertical={false}
                stroke="currentColor"
                className="text-zinc-800"
                strokeDasharray="3 3"
              />
              <XAxis
                dataKey="time"
                tickFormatter={(value) => dateFormatter.format(value)}
                minTickGap={36}
                axisLine={false}
                tickLine={false}
                tick={{ fontSize: 11, fill: "#71717a" }}
                tickMargin={12}
              />
              <YAxis
                width={56}
                tickFormatter={(value) => compactNumberFormatter.format(value)}
                axisLine={false}
                tickLine={false}
                tick={{ fontSize: 11, fill: "#71717a" }}
              />
              <Tooltip
                labelFormatter={(label) => timeFormatter.format(Number(label))}
                formatter={(value) => numberFormatter.format(Number(value))}
                contentStyle={{
                  borderRadius: 6,
                  background: "var(--color-zinc-900)",
                  borderColor: "var(--color-zinc-700)",
                  color: "white",
                  fontSize: 12,
                }}
              />
              {series.map((entry) => (
                <Line
                  key={entry.key}
                  dataKey={entry.key}
                  name={entry.name}
                  stroke={entry.color}
                  strokeWidth={2}
                  dot={{ r: 3 }}
                  activeDot={{ r: 5 }}
                  connectNulls={false}
                  isAnimationActive={false}
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        </div>
      ) : (
        <div className="flex h-64 items-center justify-center">
          <Text>No scores available for this selection.</Text>
        </div>
      )}
    </section>
  );
}
