import { MAX_RANGE_DAYS, ONE_DAY_MILLIS, parseDateInput, toDateInput } from "@/lib/loot/date";
import {
  MIN_RESOURCE_DATE,
  type ResourceDateRange,
  validResourceRange,
} from "@/lib/resources/date-range";

/** One year back from `today` (UTC), kept inside what the calendar accepts. */
export function defaultReportsRange(today: string): ResourceDateRange {
  const end = parseDateInput(today) ?? 0;
  const oneYearAgo = new Date(end);
  oneYearAgo.setUTCFullYear(oneYearAgo.getUTCFullYear() - 1);

  // The calendar allows ranges shorter than MAX_RANGE_DAYS, so a year that spans a
  // leap day starts a day later instead of being rejected.
  const longest = end - (MAX_RANGE_DAYS - 1) * ONE_DAY_MILLIS;
  let start = toDateInput(Math.max(oneYearAgo.getTime(), longest));
  if (start < MIN_RESOURCE_DATE) start = MIN_RESOURCE_DATE;

  return { start, end: today };
}

/** The range in the page address (`start` and `end`), or the default year when it is missing or invalid. */
export function resolveReportsRange(
  start: string | null,
  end: string | null,
  today: string
): ResourceDateRange {
  const range = { start: start ?? "", end: end ?? "" };

  return validResourceRange(range, today) ? range : defaultReportsRange(today);
}
