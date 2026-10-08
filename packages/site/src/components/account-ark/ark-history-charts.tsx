"use client";

import { useExtracted, useFormatter } from "next-intl";
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
  const t = useExtracted();
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
        title={t("Alliance score trend")}
        points={points}
        series={[
          { key: "own", name: t("Your alliance"), color: "#3b82f6" },
          { key: "opponent", name: t("Opponent"), color: "#f43f5e" },
        ]}
      />
      <ArkHistoryChart
        title={t("Individual score trend")}
        points={points}
        series={[{ key: "personal", name: t("Your score"), color: "#a78bfa" }]}
      />
    </div>
  );
}

function ArkHistoryChart({ title, points, series }: ArkHistoryChartProps): ReactElement {
  const intl = useFormatter();
  const t = useExtracted();
  const hasValues = points.some((point) => series.some((entry) => point[entry.key] != null));

  return (
    <section className="min-w-0 rounded-md border border-zinc-200 p-4 sm:p-5 dark:border-zinc-800">
      <Subheading>{title}</Subheading>
      <div className="mt-2 flex flex-wrap gap-4 text-xs text-zinc-500 dark:text-zinc-400">
        {series.map((entry) => (
          <span key={entry.key} className="inline-flex items-center gap-1.5">
            <span className="size-2 rounded-full" style={{ backgroundColor: entry.color }} />
            {entry.name}
          </span>
        ))}
      </div>
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
                className="text-zinc-200 dark:text-zinc-800"
                strokeDasharray="3 3"
              />
              <XAxis
                dataKey="time"
                tickFormatter={(value) =>
                  intl.dateTime(value, { month: "short", day: "numeric", timeZone: "UTC" })
                }
                minTickGap={36}
                axisLine={false}
                tickLine={false}
                tick={{ fontSize: 11, fill: "#71717a" }}
                tickMargin={12}
              />
              <YAxis
                width={56}
                tickFormatter={(value) => intl.number(value, { notation: "compact" })}
                axisLine={false}
                tickLine={false}
                tick={{ fontSize: 11, fill: "#71717a" }}
              />
              <Tooltip
                labelFormatter={(label) =>
                  intl.dateTime(Number(label), {
                    dateStyle: "medium",
                    timeStyle: "short",
                    timeZone: "UTC",
                  })
                }
                formatter={(value) => intl.number(Number(value))}
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
          <Text>{t("No scores available for this selection.")}</Text>
        </div>
      )}
    </section>
  );
}
