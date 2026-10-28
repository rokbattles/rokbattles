import type { BattleStratagem, BattleStratagemStatistic } from "@/lib/types/battle";

const decimalFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 2,
});
const decimalFormatter2 = new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 });

export function hasStratagems(
  stratagems: readonly BattleStratagem[] | null | undefined
): stratagems is readonly BattleStratagem[] {
  return Boolean(stratagems && stratagems.length > 0);
}

export function formatStratagemPercentage(value: number): string {
  return decimalFormatter.format(value);
}

export function formatStratagemStatistic(statistic: BattleStratagemStatistic): string {
  if (typeof statistic.displayValue === "number" && Number.isFinite(statistic.displayValue)) {
    const formatted = decimalFormatter.format(statistic.displayValue);
    return statistic.unit === "percent" ? `${formatted}%` : formatted;
  }

  if (typeof statistic.value === "number" && Number.isFinite(statistic.value)) {
    return decimalFormatter2.format(statistic.value);
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
