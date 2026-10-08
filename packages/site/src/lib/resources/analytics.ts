import { ONE_DAY_MILLIS, parseDateInput, toDateInput } from "@/lib/loot/date";
import { RESOURCE_KEYS, type ResourceKey } from "@/lib/resources/catalog";
import type { ResourcesDailyAggregate } from "@/lib/types/resources";

export type ResourceDay = {
  date: string;
  reports: number;
  values: Record<ResourceKey, number>;
};

export type ResourceChartPoint = { date: string } & Record<ResourceKey, number>;

const EMPTY_VALUES: Readonly<Record<ResourceKey, number>> = {
  "type:1": 0,
  "type:2": 0,
  "type:3": 0,
  "type:4": 0,
  "type:5": 0,
  crystalsGain: 0,
};

// Zeroes mean no recorded resources; future days are excluded entirely.
export function buildResourceDays(
  daily: ResourcesDailyAggregate[],
  start: string,
  end: string,
  today: string
): ResourceDay[] {
  const first = parseDateInput(start);
  const last = parseDateInput(end < today ? end : today);

  if (first == null || last == null || last < first) {
    return [];
  }

  const byDate = new Map(daily.map((day) => [day.date, day]));
  const days: ResourceDay[] = [];

  for (let cursor = first; cursor <= last; cursor += ONE_DAY_MILLIS) {
    const date = toDateInput(cursor);
    const day = byDate.get(date);
    const values = { ...EMPTY_VALUES, crystalsGain: day?.crystalsGain ?? 0 };

    for (const resource of day?.resources ?? []) {
      const key = `type:${resource.type}` as ResourceKey;

      if (key in values) {
        values[key] += resource.total;
      }
    }

    days.push({ date, reports: day?.reports ?? 0, values });
  }

  return days;
}

export function buildResourceChartPoints(
  days: ResourceDay[],
  cumulative: boolean
): ResourceChartPoint[] {
  const running = { ...EMPTY_VALUES };

  return days.map((day) => {
    for (const key of RESOURCE_KEYS) {
      running[key] += day.values[key];
    }

    return { date: day.date, ...(cumulative ? running : day.values) };
  });
}

export function resourceLedgerCsv(days: ResourceDay[]): string {
  const escapeCell = (value: string | number): string => `"${String(value).replaceAll('"', '""')}"`;
  const rows: (string | number)[][] = [
    ["Date", "Reports", "Food", "Wood", "Stone", "Gold", "Gems", "Crystals"],
    ...days.map((day) => [day.date, day.reports, ...RESOURCE_KEYS.map((key) => day.values[key])]),
  ];

  return rows.map((row) => row.map(escapeCell).join(",")).join("\r\n");
}
