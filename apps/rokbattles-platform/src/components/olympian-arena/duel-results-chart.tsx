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
import { DuelSummaryTooltip } from "@/components/olympian-arena/duel-summary-tooltip";
import type { DuelBattle2BattleResult, DuelBattle2BattleResults } from "@/lib/types/duelbattle2";

type DuelMetricConfig = {
  labelKey: string;
  valueKey: keyof DuelBattle2BattleResult;
  commonLabel?: boolean;
};

const COMMON_METRIC_KEYS = new Set(["units", "dead", "severelyWounded", "killPoints"]);

const DUEL_METRICS: readonly DuelMetricConfig[] = [
  { labelKey: "units", valueKey: "units", commonLabel: true },
  { labelKey: "dead", valueKey: "dead", commonLabel: true },
  {
    labelKey: "severelyWounded",
    valueKey: "severely_wounded",
    commonLabel: true,
  },
  { labelKey: "wounded", valueKey: "slightly_wounded" },
  { labelKey: "healed", valueKey: "heal" },
  { labelKey: "killPoints", valueKey: "kill_points", commonLabel: true },
  { labelKey: "power", valueKey: "power" },
] as const;

type DuelSummaryDatum = {
  key: string;
  label: string;
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

function buildChartData(
  results: DuelBattle2BattleResults,
  t: ReturnType<typeof useTranslations>,
  tCommon: ReturnType<typeof useTranslations>
) {
  const rows: DuelSummaryDatum[] = [];
  for (const metric of DUEL_METRICS) {
    const senderValue = getMetricValue(results.sender, metric.valueKey);
    const opponentValue = getMetricValue(results.opponent, metric.valueKey);
    if (senderValue == null && opponentValue == null) {
      continue;
    }
    const label =
      metric.commonLabel || COMMON_METRIC_KEYS.has(metric.labelKey)
        ? tCommon(`metrics.${metric.labelKey}`)
        : t(`metrics.${metric.labelKey}`);

    rows.push({
      key: metric.labelKey,
      label,
      sender: senderValue ?? 0,
      opponent: opponentValue ?? 0,
    });
  }
  return rows;
}

export function DuelResultsChart({ results }: { results: DuelBattle2BattleResults }) {
  const t = useTranslations("duels");
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
              dataKey="label"
              width={150}
              tick={{ fontSize: 12, fill: "#6b7280" }}
              axisLine={{ stroke: "#d4d4d8" }}
              tickLine={false}
            />
            <RechartsTooltip
              cursor={{ fill: "rgba(39, 39, 42, 0.08)" }}
              content={(props) => (
                <DuelSummaryTooltip
                  active={props.active}
                  payload={props.payload}
                  label={props.label}
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
