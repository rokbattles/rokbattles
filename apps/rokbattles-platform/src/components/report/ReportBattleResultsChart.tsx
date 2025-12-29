import {
  Bar,
  BarChart,
  CartesianGrid,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  XAxis,
  YAxis,
} from "recharts";
import { ReportBattleSummaryTooltip } from "@/components/report/ReportBattleSummaryTooltip";
import type { RawBattleResults } from "@/lib/types/rawReport";

type BattleMetricConfig = {
  label: string;
  selfKey: keyof RawBattleResults;
  enemyKey: keyof RawBattleResults;
};

const BATTLE_METRICS: readonly BattleMetricConfig[] = [
  { label: "Units", selfKey: "max", enemyKey: "enemy_max" },
  { label: "Remaining", selfKey: "remaining", enemyKey: "enemy_remaining" },
  { label: "Heal", selfKey: "healing", enemyKey: "enemy_healing" },
  { label: "Dead", selfKey: "death", enemyKey: "enemy_death" },
  {
    label: "Severely wounded",
    selfKey: "severely_wounded",
    enemyKey: "enemy_severely_wounded",
  },
  { label: "Slightly wounded", selfKey: "wounded", enemyKey: "enemy_wounded" },
  { label: "Watchtower Damage", selfKey: "watchtower", enemyKey: "enemy_watchtower" },
  { label: "Kill Points", selfKey: "kill_score", enemyKey: "enemy_kill_score" },
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
      key: metric.label,
      label: metric.label,
      self: selfValue ?? 0,
      enemy: enemyValue ?? 0,
    });
  }
  return rows;
}

export function ReportBattleResultsChart({ results }: { results: RawBattleResults }) {
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
              dataKey="label"
              width={140}
              tick={{ fontSize: 12, fill: "#6b7280" }}
              axisLine={{ stroke: "#d4d4d8" }}
              tickLine={false}
            />
            <RechartsTooltip
              cursor={{ fill: "rgba(39, 39, 42, 0.08)" }}
              content={(props) => (
                <ReportBattleSummaryTooltip
                  active={props.active}
                  payload={props.payload}
                  label={props.label}
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
