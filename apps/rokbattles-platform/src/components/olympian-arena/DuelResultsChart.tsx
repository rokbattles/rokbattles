import {
  Bar,
  BarChart,
  CartesianGrid,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  XAxis,
  YAxis,
} from "recharts";
import { DuelSummaryTooltip } from "@/components/olympian-arena/DuelSummaryTooltip";
import type { DuelResults } from "@/lib/types/duelReport";

type DuelMetricConfig = {
  label: string;
  senderKey: keyof DuelResults;
  opponentKey: keyof DuelResults;
};

const DUEL_METRICS: readonly DuelMetricConfig[] = [
  { label: "Units", senderKey: "units", opponentKey: "opponent_units" },
  { label: "Dead", senderKey: "dead", opponentKey: "opponent_dead" },
  {
    label: "Severely wounded",
    senderKey: "sev_wounded",
    opponentKey: "opponent_sev_wounded",
  },
  { label: "Wounded", senderKey: "wounded", opponentKey: "opponent_wounded" },
  { label: "Healed", senderKey: "heal", opponentKey: "opponent_heal" },
  { label: "Kill Points", senderKey: "kill_points", opponentKey: "opponent_kill_points" },
  { label: "Power", senderKey: "power", opponentKey: "opponent_power" },
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
  const raw = results?.[key];
  if (typeof raw !== "number" || !Number.isFinite(raw)) {
    return null;
  }
  return raw;
}

function buildChartData(results: DuelResults) {
  const rows: DuelSummaryDatum[] = [];
  for (const metric of DUEL_METRICS) {
    const senderValue = getMetricValue(results, metric.senderKey);
    const opponentValue = getMetricValue(results, metric.opponentKey);
    if (senderValue == null && opponentValue == null) {
      continue;
    }

    rows.push({
      key: metric.label,
      label: metric.label,
      sender: senderValue ?? 0,
      opponent: opponentValue ?? 0,
    });
  }
  return rows;
}

export function DuelResultsChart({ results }: { results: DuelResults }) {
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
