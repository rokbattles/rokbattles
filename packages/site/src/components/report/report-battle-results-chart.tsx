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
import { ReportBattleSummaryTooltip } from "@/components/report/report-battle-summary-tooltip";
import { GameTranslate } from "@/components/v1/game-translate";
import type { RawBattleResults } from "@/lib/types/raw-report";

type BattleMetricConfig = {
  labelKey: string;
  selfKey: keyof RawBattleResults;
  enemyKey: keyof RawBattleResults;
};

const BATTLE_METRICS: readonly BattleMetricConfig[] = [
  { labelKey: "units", selfKey: "max", enemyKey: "enemy_max" },
  { labelKey: "remaining", selfKey: "remaining", enemyKey: "enemy_remaining" },
  { labelKey: "heal", selfKey: "healing", enemyKey: "enemy_healing" },
  { labelKey: "dead", selfKey: "death", enemyKey: "enemy_death" },
  {
    labelKey: "severelyWounded",
    selfKey: "severely_wounded",
    enemyKey: "enemy_severely_wounded",
  },
  { labelKey: "slightlyWounded", selfKey: "wounded", enemyKey: "enemy_wounded" },
  { labelKey: "killPoints", selfKey: "kill_score", enemyKey: "enemy_kill_score" },
  { labelKey: "acclaim", selfKey: "acclaim", enemyKey: "enemy_acclaim" },
] as const;

type BattleSummaryDatum = {
  key: string;
  self: number;
  enemy: number;
};

const numberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

function getMetricValue(results: RawBattleResults, key: keyof RawBattleResults) {
  const raw = results?.[key];
  if (typeof raw !== "number" || !Number.isFinite(raw)) {
    return null;
  }
  return raw;
}

function buildChartData(results: RawBattleResults) {
  const rows: BattleSummaryDatum[] = [];
  for (const metric of BATTLE_METRICS) {
    const selfValue = getMetricValue(results, metric.selfKey);
    const enemyValue = getMetricValue(results, metric.enemyKey);
    if (selfValue == null && enemyValue == null) {
      continue;
    }
    rows.push({
      key: metric.labelKey,
      self: selfValue ?? 0,
      enemy: enemyValue ?? 0,
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

export function ReportBattleResultsChart({ results }: { results: RawBattleResults }) {
  const t = useExtracted();
  const chartLabels: Record<string, ReactNode> = {
    units: t("Units"),
    remaining: <GameTranslate value="LC_COMMON_UNITS_REMAINING" />,
    heal: <GameTranslate value="LC_OTHER_BATTLEREPORT_STATISTICS_HEAL" />,
    dead: <GameTranslate value="LC_COMMON_DEATH" />,
    severelyWounded: <GameTranslate value="LC_COMMON_SEVERELY_WOUNDED" />,
    slightlyWounded: <GameTranslate value="LC_COMMON_SLIGHTLY_WOUNDED" />,
    killPoints: <GameTranslate value="LC_COMMON_KILL_SCORE" />,
    acclaim: <GameTranslate value="LC_COMMON_CURRENT_CONTRIBUTION" />,
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
              width={140}
              tick={(props) => <ChartLabelTick {...props} labels={chartLabels} />}
              axisLine={{ stroke: "#d4d4d8" }}
              tickLine={false}
            />
            <RechartsTooltip
              cursor={{ fill: "rgba(39, 39, 42, 0.08)" }}
              content={(props) => (
                <ReportBattleSummaryTooltip
                  active={props.active}
                  payload={props.payload}
                  label={chartLabels[String(props.label)] ?? props.label}
                />
              )}
            />
            <Bar
              dataKey="self"
              stackId="battle"
              fill="#3b82f6"
              radius={[4, 0, 0, 4]}
              maxBarSize={28}
            />
            <Bar
              dataKey="enemy"
              stackId="battle"
              fill="#f87171"
              radius={[0, 4, 4, 0]}
              maxBarSize={28}
            />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
