import { formatWholeNumber } from "@/lib/loot/format";

export function formatArkMetricValue(value: number | null, fallbackLabel: string): string {
  if (value == null) {
    return fallbackLabel;
  }

  return formatWholeNumber(value);
}

export function formatArkPercentValue(value: number | null, fallbackLabel: string): string {
  if (value == null) {
    return fallbackLabel;
  }

  return `${formatWholeNumber(value)}%`;
}

export function formatArkBattlesValue(
  battlesWin: number | null,
  battlesTotal: number | null,
  fallbackLabel: string
): string {
  if (battlesWin == null && battlesTotal == null) {
    return fallbackLabel;
  }

  return `${formatWholeNumber(battlesWin ?? 0)} / ${formatWholeNumber(battlesTotal ?? 0)}`;
}
