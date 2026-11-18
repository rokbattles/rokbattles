import type { PairingDurationDistribution, PairingPerformanceResponse } from "@/lib/pairings";

const secondsFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });

export const DURATION_PERCENTILES = [
  ["p10Millis", "P10"],
  ["p25Millis", "P25"],
  ["p50Millis", "P50 · Median"],
  ["p75Millis", "P75"],
  ["p90Millis", "P90"],
] as const;

export function formatBattleLength(millis: number | null | undefined) {
  if (millis == null) return "—";

  const seconds = Math.round(Math.max(0, millis) / 100) / 10;
  if (seconds < 60) return `${secondsFormatter.format(seconds)}s`;

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${secondsFormatter.format(seconds % 60)}s`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}

export type DurationPoint = {
  date: string;
  battles: number;
  totalMillis: number;
  distribution: PairingDurationDistribution | null;
  outerRange: [number, number] | null;
  innerRange: [number, number] | null;
  medianMillis: number | null;
};

export function durationPoints(series: PairingPerformanceResponse["series"]): DurationPoint[] {
  return series.map(({ date, metrics }) => {
    const distribution = metrics.durationDistribution;

    return {
      date,
      battles: metrics.battles,
      totalMillis: metrics.battleDurationMillis,
      distribution,
      outerRange: distribution ? [distribution.p10Millis, distribution.p90Millis] : null,
      innerRange: distribution ? [distribution.p25Millis, distribution.p75Millis] : null,
      medianMillis: distribution?.p50Millis ?? null,
    };
  });
}
