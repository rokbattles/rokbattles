const FALLBACK_UTC_DISPLAY = "UTC --/-- --:--";

type LongLike = { $numberLong: string };
type DateLike = { $date: string | LongLike };
export type DateInput = number | string | Date | LongLike | DateLike | null | undefined;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function toEpochMillis(value: DateInput): number | null {
  if (value == null) {
    return null;
  }

  if (value instanceof Date) {
    return value.getTime();
  }

  if (typeof value === "number") {
    return Number.isFinite(value) ? value : null;
  }

  if (typeof value === "string") {
    const trimmed = value.trim();
    if (trimmed === "") {
      return null;
    }

    const numeric = Number(trimmed);
    if (Number.isFinite(numeric)) {
      return numeric;
    }

    const parsed = Date.parse(trimmed);
    return Number.isNaN(parsed) ? null : parsed;
  }

  if (isRecord(value)) {
    const numberLike = value as LongLike;
    if (typeof numberLike.$numberLong === "string") {
      const numeric = Number(numberLike.$numberLong);
      return Number.isFinite(numeric) ? numeric : null;
    }

    const dateLike = value as DateLike;
    if (dateLike.$date) {
      if (typeof dateLike.$date === "string") {
        const parsed = Date.parse(dateLike.$date);
        return Number.isNaN(parsed) ? null : parsed;
      }

      if (isRecord(dateLike.$date)) {
        const innerNumeric = (dateLike.$date as LongLike).$numberLong;
        if (typeof innerNumeric === "string") {
          const numeric = Number(innerNumeric);
          return Number.isFinite(numeric) ? numeric : null;
        }
      }
    }
  }

  return null;
}

function normalizeEpoch(value: number): number {
  const absValue = Math.abs(value);
  if (absValue < 1e12) {
    return value * 1000;
  }
  if (absValue >= 1e17) {
    return value / 1e6;
  }
  if (absValue >= 1e14) {
    return value / 1e3;
  }
  return value;
}

function toDate(value: DateInput): Date | null {
  const epoch = toEpochMillis(value);
  if (epoch == null) {
    return null;
  }

  const normalized = normalizeEpoch(epoch);
  if (normalized <= 0) {
    return null;
  }

  const date = new Date(normalized);
  return Number.isNaN(date.getTime()) ? null : date;
}

const durationUnits = [
  { label: "d", millis: 24 * 60 * 60 * 1000 },
  { label: "h", millis: 60 * 60 * 1000 },
  { label: "m", millis: 60 * 1000 },
  { label: "s", millis: 1000 },
] as const;

export function formatDurationShort(start: DateInput, end: DateInput): string {
  const startDate = toDate(start);
  const endDate = toDate(end);

  if (!startDate || !endDate) {
    return "0s";
  }

  let remaining = Math.max(0, endDate.getTime() - startDate.getTime());

  const parts: string[] = [];

  for (const unit of durationUnits) {
    if (remaining >= unit.millis) {
      const value = Math.floor(remaining / unit.millis);
      parts.push(`${value}${unit.label}`);
      remaining -= value * unit.millis;
    }
  }

  if (parts.length === 0) {
    parts.push("0s");
  }

  return parts.join(" ");
}

function pad(value: number): string {
  return value.toString().padStart(2, "0");
}

export function formatUtcDateTime(value: DateInput): string {
  const date = toDate(value);

  if (!date) {
    return FALLBACK_UTC_DISPLAY;
  }

  const month = pad(date.getUTCMonth() + 1);
  const day = pad(date.getUTCDate());
  const hours = pad(date.getUTCHours());
  const minutes = pad(date.getUTCMinutes());

  return `UTC ${month}/${day} ${hours}:${minutes}`;
}
