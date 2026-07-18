import type { BattleStratagem, BattleStratagemStatistic } from "@/lib/types/battle";

export function hasStratagems(
  stratagems: readonly BattleStratagem[] | null | undefined
): stratagems is readonly BattleStratagem[] {
  return Boolean(stratagems && stratagems.length > 0);
}

export function formatStratagemPercentage(value: number, locale?: string): string {
  return new Intl.NumberFormat(locale, {
    maximumFractionDigits: 2,
  }).format(value);
}

export function formatStratagemStatistic(
  statistic: BattleStratagemStatistic,
  locale?: string
): string {
  if (typeof statistic.displayValue === "number" && Number.isFinite(statistic.displayValue)) {
    const formatted = new Intl.NumberFormat(locale, {
      maximumFractionDigits: 2,
    }).format(statistic.displayValue);
    return statistic.unit === "percent" ? `${formatted}%` : formatted;
  }

  if (typeof statistic.value === "number" && Number.isFinite(statistic.value)) {
    return new Intl.NumberFormat(locale, { maximumFractionDigits: 2 }).format(statistic.value);
  }
  if (typeof statistic.value === "string") {
    return statistic.value;
  }
  if (statistic.value == null) {
    return "—";
  }

  try {
    return JSON.stringify(statistic.value);
  } catch {
    return String(statistic.value);
  }
}
