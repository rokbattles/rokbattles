export const ONE_DAY_MILLIS = 24 * 60 * 60 * 1000;
export const MAX_RANGE_DAYS = 366;

export function parseDateInput(value: string): number | null {
  if (!value) {
    return null;
  }

  const parsed = new Date(`${value}T00:00:00Z`);
  const millis = parsed.getTime();
  return Number.isNaN(millis) ? null : millis;
}

export function toDateInput(millis: number): string {
  return new Date(millis).toISOString().slice(0, 10);
}

export function toDateLabel(value: string): string {
  const parsed = new Date(`${value}T00:00:00Z`);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  }).format(parsed);
}

export function todayUtcStartMillis(): number {
  const now = new Date();
  return Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
}

export function toDateKey(millis: number): string {
  return new Date(millis).toISOString().slice(0, 10);
}
