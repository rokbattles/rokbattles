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
import type { DuelResults } from "@/lib/types/duel-report";

type DuelMetricConfig = {
  labelKey: string;
  senderKey: keyof DuelResults;
  opponentKey: keyof DuelResults;
};

const COMMON_METRIC_KEYS = new Set([
  "units",
  "dead",
  "severelyWounded",
  "killPoints",
]);

const DUEL_METRICS: readonly DuelMetricConfig[] = [
  { labelKey: "units", senderKey: "units", opponentKey: "opponent_units" },
  { labelKey: "dead", senderKey: "dead", opponentKey: "opponent_dead" },
  {
    labelKey: "severelyWounded",
    senderKey: "sev_wounded",
    opponentKey: "opponent_sev_wounded",
  },
  {
    labelKey: "wounded",
    senderKey: "wounded",
    opponentKey: "opponent_wounded",
  },
  { labelKey: "healed", senderKey: "heal", opponentKey: "opponent_heal" },
  {
    labelKey: "killPoints",
    senderKey: "kill_points",
    opponentKey: "opponent_kill_points",
  },
  { labelKey: "power", senderKey: "power", opponentKey: "opponent_power" },
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

function getMetricValue(results: DuelResults, key: keyof DuelResults) {
  const raw = results[key];
  if (typeof raw !== "number" || !Number.isFinite(raw)) {
    return null;
  }
  return raw;
}

function buildChartData(
  results: DuelResults,
  t: ReturnType<typeof useTranslations>,
  tCommon: ReturnType<typeof useTranslations>
) {
  const rows: DuelSummaryDatum[] = [];
  for (const metric of DUEL_METRICS) {
    const senderValue = getMetricValue(results, metric.senderKey);
    const opponentValue = getMetricValue(results, metric.opponentKey);
    if (senderValue == null && opponentValue == null) {
      continue;
    }
    const label = COMMON_METRIC_KEYS.has(metric.labelKey)
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

export function DuelResultsChart({ results }: { results: DuelResults }) {
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
              width={150}
            />
            <RechartsTooltip
              content={(props) => (
                <DuelSummaryTooltip
                  active={props.active}
                  label={props.label}
                  payload={props.payload}
                />
              )}
              cursor={{ fill: "rgba(39, 39, 42, 0.08)" }}
            />
            <Bar
              dataKey="sender"
              fill="#3b82f6"
              radius={[4, 0, 0, 4]}
              stackId="duel"
            />
            <Bar
              dataKey="opponent"
              fill="#f87171"
              radius={[0, 4, 4, 0]}
              stackId="duel"
            />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
