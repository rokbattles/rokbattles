import { decimalFormatter } from "@/lib/statistics-format";

export { decimalFormatter, formatPerSecond } from "@/lib/statistics-format";

export const numberFormatter = new Intl.NumberFormat("en-US");

export const scoreFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 2,
  minimumFractionDigits: 2,
});

const dateFormatter = new Intl.DateTimeFormat("en-US", {
  day: "2-digit",
  hour: "2-digit",
  hourCycle: "h23",
  minute: "2-digit",
  month: "2-digit",
  timeZone: "UTC",
});

export function clampScore(value: number): number {
  return Number.isFinite(value) ? Math.min(10, Math.max(0, value)) : 0;
}

export function formatNumber(value: number): string {
  return numberFormatter.format(Math.round(Number.isFinite(value) ? value : 0));
}

export function formatPercent(value: number): string {
  return `${decimalFormatter.format(Number.isFinite(value) ? value : 0)}%`;
}

export function formatDuration(valueMillis: number): string {
  if (!Number.isFinite(valueMillis) || valueMillis <= 0) {
    return "0s";
  }

  const totalSeconds = Math.round(valueMillis / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;

  if (minutes <= 0) {
    return `${seconds}s`;
  }

  return `${minutes}m ${seconds}s`;
}

export function formatRefreshedAt(value: string): string {
  return `${dateFormatter.format(new Date(value)).replace(",", "")} UTC`;
}
