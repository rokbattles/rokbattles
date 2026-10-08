import { MAX_RANGE_DAYS, ONE_DAY_MILLIS, parseDateInput, toDateInput } from "@/lib/loot/date";

export const MIN_RESOURCE_DATE = "2025-01-01";

export type ResourceDateRange = { start: string; end: string };

type ResourceDatePreset = "last7" | "last30" | "last90" | "ytd";

export function resourceCalendarMonth(date: string, offset: number): string {
  const year = Number(date.slice(0, 4));
  const month = Number(date.slice(5, 7)) - 1 + offset;

  return toDateInput(Date.UTC(year, month, 1));
}

export function resourcePresetRange(preset: ResourceDatePreset, today: string): ResourceDateRange {
  if (preset === "ytd") {
    return { start: `${today.slice(0, 4)}-01-01`, end: today };
  }

  const days = { last7: 7, last30: 30, last90: 90 }[preset];
  const start = toDateInput((parseDateInput(today) ?? 0) - (days - 1) * ONE_DAY_MILLIS);

  return { start: start < MIN_RESOURCE_DATE ? MIN_RESOURCE_DATE : start, end: today };
}

export function validResourceRange(range: ResourceDateRange, today: string): boolean {
  const start = parseDateInput(range.start);
  const end = parseDateInput(range.end);

  return (
    start != null &&
    end != null &&
    toDateInput(start) === range.start &&
    toDateInput(end) === range.end &&
    range.start >= MIN_RESOURCE_DATE &&
    range.end <= today &&
    end >= start &&
    end - start < MAX_RANGE_DAYS * ONE_DAY_MILLIS
  );
}

export function resolveResourceRange(
  start: string | null,
  end: string | null,
  today: string
): ResourceDateRange {
  const range = { start: start ?? "", end: end ?? "" };

  return validResourceRange(range, today) ? range : resourcePresetRange("last30", today);
}
