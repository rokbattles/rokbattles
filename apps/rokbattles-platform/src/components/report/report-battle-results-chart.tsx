"use client";

import { useTranslations } from "next-intl";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  XAxis,
  YAxis,
} from "recharts";
import { ReportBattleSummaryTooltip } from "@/components/report/report-battle-summary-tooltip";
import type { RawBattleResults } from "@/lib/types/raw-report";

type BattleMetricConfig = {
  labelKey: string;
  selfKey: keyof RawBattleResults;
  enemyKey: keyof RawBattleResults;
};

const COMMON_METRIC_KEYS = new Set([
  "units",
  "remaining",
  "dead",
  "severelyWounded",
  "killPoints",
]);

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
  {
    labelKey: "slightlyWounded",
    selfKey: "wounded",
    enemyKey: "enemy_wounded",
  },
  {
    labelKey: "watchtowerDamage",
    selfKey: "watchtower",
    enemyKey: "enemy_watchtower",
  },
  {
    labelKey: "killPoints",
    selfKey: "kill_score",
    enemyKey: "enemy_kill_score",
  },
] as const;

type BattleSummaryDatum = {
  key: string;
  label: string;
  self: number;
  enemy: number;
};

const numberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

function getMetricValue(
  results: RawBattleResults,
  key: keyof RawBattleResults
) {
  const raw = results?.[key];
  if (typeof raw !== "number" || !Number.isFinite(raw)) {
    return null;
  }
  return raw;
}

function buildChartData(
  results: RawBattleResults,
  t: ReturnType<typeof useTranslations>,
  tCommon: ReturnType<typeof useTranslations>
) {
  const rows: BattleSummaryDatum[] = [];
  for (const metric of BATTLE_METRICS) {
    const selfValue = getMetricValue(results, metric.selfKey);
    const enemyValue = getMetricValue(results, metric.enemyKey);
    if (selfValue == null && enemyValue == null) {
      continue;
    }
    const label = COMMON_METRIC_KEYS.has(metric.labelKey)
      ? tCommon(`metrics.${metric.labelKey}`)
      : t(`metrics.battle.${metric.labelKey}`);

    rows.push({
      key: metric.labelKey,
      label,
      self: selfValue ?? 0,
      enemy: enemyValue ?? 0,
    });
  }
  return rows;
}

export function ReportBattleResultsChart({
  results,
}: {
  results: RawBattleResults;
}) {
  const t = useTranslations("report");
  const tCommon = useTranslations("common");
  const chartData = buildChartData(results, t, tCommon);

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
            <CartesianGrid
              horizontal={false}
              stroke="#d4d4d8"
              strokeDasharray="3 3"
            />
            <XAxis
              axisLine={{ stroke: "#d4d4d8" }}
              tick={{ fontSize: 12, fill: "#6b7280" }}
              tickFormatter={(value) => numberFormatter.format(value)}
              tickLine={false}
              type="number"
            />
            <YAxis
              axisLine={{ stroke: "#d4d4d8" }}
              dataKey="label"
              tick={{ fontSize: 12, fill: "#6b7280" }}
              tickLine={false}
              type="category"
              width={140}
            />
            <RechartsTooltip
              content={(props) => (
                <ReportBattleSummaryTooltip
                  active={props.active}
                  label={props.label}
                  payload={props.payload}
                />
              )}
              cursor={{ fill: "rgba(39, 39, 42, 0.08)" }}
            />
            <Bar
              dataKey="self"
              fill="#3b82f6"
              maxBarSize={28}
              radius={[4, 0, 0, 4]}
              stackId="battle"
            />
            <Bar
              dataKey="enemy"
              fill="#f87171"
              maxBarSize={28}
              radius={[0, 4, 4, 0]}
              stackId="battle"
            />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
