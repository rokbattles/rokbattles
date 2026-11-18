import type { PairingPerformanceMetrics } from "@/lib/pairings";
import { formatCompact, formatExact } from "@/lib/pairings/analytics";

type PerformanceFormat = "number" | "duration" | "percent" | "rate";

export type PerformanceChart = {
  id: string;
  label: string;
  format: PerformanceFormat;
  series: {
    key: Exclude<keyof PairingPerformanceMetrics, "durationDistribution">;
    label: string;
    totalLabel?: string;
    color: string;
  }[];
};

export const PERFORMANCE_CHARTS: PerformanceChart[] = [
  {
    id: "battles",
    label: "Battles",
    format: "number",
    series: [{ key: "battles", label: "Battles", color: "#60a5fa" }],
  },
  {
    id: "kp",
    label: "KP Gain/Loss",
    format: "number",
    series: [
      { key: "kpGain", label: "KP Gain", totalLabel: "Gain", color: "#60a5fa" },
      { key: "kpLoss", label: "KP Loss", totalLabel: "Loss", color: "#fb7185" },
    ],
  },
  {
    id: "sevs",
    label: "Sevs Gain/Loss",
    format: "number",
    series: [
      { key: "sevsGain", label: "Sevs Gain", totalLabel: "Gain", color: "#60a5fa" },
      { key: "sevsLoss", label: "Sevs Loss", totalLabel: "Loss", color: "#fb7185" },
    ],
  },
  {
    id: "power",
    label: "Power Loss",
    format: "number",
    series: [
      { key: "powerLoss", label: "Your power loss", totalLabel: "Yours", color: "#fb7185" },
      {
        key: "opponentPowerLoss",
        label: "Opponent power loss",
        totalLabel: "Opponents",
        color: "#60a5fa",
      },
    ],
  },
  {
    id: "trade",
    label: "KP Trade %",
    format: "percent",
    series: [{ key: "tradePercent", label: "KP Trade %", color: "#34d399" }],
  },
];

export const TEMPO_CHARTS: PerformanceChart[] = [
  {
    id: "dps",
    label: "DPS",
    format: "rate",
    series: [{ key: "dps", label: "Damage per second", color: "#60a5fa" }],
  },
  {
    id: "sps",
    label: "SPS",
    format: "rate",
    series: [{ key: "sps", label: "Severely wounded inflicted per second", color: "#a78bfa" }],
  },
  {
    id: "tps",
    label: "TPS",
    format: "rate",
    series: [{ key: "tps", label: "Severely wounded taken per second", color: "#fb7185" }],
  },
  {
    id: "hps",
    label: "HPS",
    format: "rate",
    series: [{ key: "hps", label: "Healing per second", color: "#34d399" }],
  },
];

const rateFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 });

export function formatBattleDuration(millis: number) {
  const seconds = Math.floor(Math.max(0, millis) / 1000);
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  if (hours) return `${formatExact(hours)}h ${minutes}m`;
  if (minutes) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
}

export function formatPerformanceValue(
  value: number | null,
  format: PerformanceFormat,
  exact = false
) {
  if (value == null) return "—";
  if (format === "duration") return formatBattleDuration(value);
  if (format === "percent") return `${value.toFixed(1)}%`;
  if (format === "rate")
    return `${exact || Math.abs(value) < 1_000 ? rateFormatter.format(value) : formatCompact(value)}/s`;

  return exact ? formatExact(value) : formatCompact(value);
}

export function formatPerformanceAxis(value: number, format: PerformanceFormat) {
  if (format !== "duration") return formatPerformanceValue(value, format);

  if (value >= 3_600_000) return `${formatCompact(value / 3_600_000)}h`;
  if (value >= 60_000) return `${formatCompact(value / 60_000)}m`;
  return `${formatCompact(value / 1000)}s`;
}
