"use client";

import { useExtracted } from "next-intl";
import type { ReactNode } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  XAxis,
  YAxis,
  type YAxisTickContentProps,
} from "recharts";
import { DuelSummaryTooltip } from "@/components/olympian-arena/duel-summary-tooltip";
import { GameTranslate } from "@/components/v1/game-translate";
import type { DuelBattle2BattleResult, DuelBattle2BattleResults } from "@/lib/types/duelbattle2";

type DuelMetricConfig = {
  labelKey: string;
  valueKey: keyof DuelBattle2BattleResult;
};

const DUEL_METRICS: readonly DuelMetricConfig[] = [
  { labelKey: "units", valueKey: "units" },
  { labelKey: "dead", valueKey: "dead" },
  {
    labelKey: "severelyWounded",
    valueKey: "severelyWounded",
  },
  { labelKey: "wounded", valueKey: "slightlyWounded" },
  { labelKey: "healed", valueKey: "heal" },
  { labelKey: "killPoints", valueKey: "killPoints" },
  { labelKey: "power", valueKey: "power" },
] as const;

type DuelSummaryDatum = {
  key: string;
  sender: number;
  opponent: number;
};

const numberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

function getMetricValue(results: DuelBattle2BattleResult, key: keyof DuelBattle2BattleResult) {
  const raw = results[key];
  if (typeof raw !== "number" || !Number.isFinite(raw)) {
    return null;
  }
  return raw;
}

function buildChartData(results: DuelBattle2BattleResults) {
  const rows: DuelSummaryDatum[] = [];
  for (const metric of DUEL_METRICS) {
    const senderValue = getMetricValue(results.sender, metric.valueKey);
    const opponentValue = getMetricValue(results.opponent, metric.valueKey);
    if (senderValue == null && opponentValue == null) {
      continue;
    }
    rows.push({
      key: metric.labelKey,
      sender: senderValue ?? 0,
      opponent: opponentValue ?? 0,
    });
  }
  return rows;
}

function ChartLabelTick({
  labels,
  payload,
  x,
  y,
}: YAxisTickContentProps & {
  labels: Record<string, ReactNode>;
}) {
  return (
    <text x={x} y={y} dy="0.32em" fill="#6b7280" fontSize={12} textAnchor="end">
      {labels[String(payload.value)]}
    </text>
  );
}

export function DuelResultsChart({ results }: { results: DuelBattle2BattleResults }) {
  const t = useExtracted();
  const chartLabels: Record<string, ReactNode> = {
    units: t("Units"),
    dead: <GameTranslate value="LC_COMMON_DEATH" />,
    severelyWounded: <GameTranslate value="LC_COMMON_SEVERELY_WOUNDED" />,
    wounded: <GameTranslate value="LC_COMMON_THE_WOUNDED" />,
    healed: <GameTranslate value="LC_BATTLEFIELD_JJC_STATS_HEAL" />,
    killPoints: <GameTranslate value="LC_COMMON_KILL_SCORE" />,
    power: <GameTranslate value="LC_COMMON_POWER" />,
  };
  const chartData = buildChartData(results);

  if (chartData.length === 0) {
    return null;
  }

  return (
    <div className="space-y-4">
      <div className="h-[320px] w-full">
        <ResponsiveContainer>
          <BarChart
            data={chartData}
            layout="vertical"
            margin={{ top: 12, right: 16, bottom: 12, left: 4 }}
          >
            <CartesianGrid strokeDasharray="3 3" horizontal={false} stroke="#d4d4d8" />
            <XAxis
              type="number"
              tickFormatter={(value) => numberFormatter.format(value)}
              tick={{ fontSize: 12, fill: "#6b7280" }}
              axisLine={{ stroke: "#d4d4d8" }}
              tickLine={false}
            />
            <YAxis
              type="category"
              dataKey="key"
              width={150}
              tick={(props) => <ChartLabelTick {...props} labels={chartLabels} />}
              axisLine={{ stroke: "#d4d4d8" }}
              tickLine={false}
            />
            <RechartsTooltip
              cursor={{ fill: "rgba(39, 39, 42, 0.08)" }}
              content={(props) => (
                <DuelSummaryTooltip
                  active={props.active}
                  payload={props.payload}
                  label={chartLabels[String(props.label)] ?? props.label}
                />
              )}
            />
            <Bar dataKey="sender" stackId="duel" fill="#3b82f6" radius={[4, 0, 0, 4]} />
            <Bar dataKey="opponent" stackId="duel" fill="#f87171" radius={[0, 4, 4, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
