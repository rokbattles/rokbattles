const FALLBACK_UTC_DISPLAY = "UTC --/-- --:--";

export type TimestampInput = number | null | undefined;

function pad(value: number): string {
  return value.toString().padStart(2, "0");
}

export function normalizeEpochMillis(value: number): number {
  const absoluteValue = Math.abs(value);

  if (absoluteValue >= 1e17) {
    return Math.trunc(value / 1e6);
  }

  if (absoluteValue >= 1e14) {
    return Math.trunc(value / 1e3);
  }

  if (absoluteValue < 1e12) {
    return Math.trunc(value * 1000);
  }

  return Math.trunc(value);
}

export function toEpochMillis(value: TimestampInput): number | null {
  if (value == null || !Number.isFinite(value)) {
    return null;
  }

  return normalizeEpochMillis(value);
}

export function formatUtcDateTime(value: TimestampInput): string {
  const millis = toEpochMillis(value);

  if (millis == null) {
    return FALLBACK_UTC_DISPLAY;
  }

  const date = new Date(millis);

  if (Number.isNaN(date.getTime())) {
    return FALLBACK_UTC_DISPLAY;
  }

  const month = pad(date.getUTCMonth() + 1);
  const day = pad(date.getUTCDate());
  const hour = pad(date.getUTCHours());
  const minute = pad(date.getUTCMinutes());

  return `UTC ${month}/${day} ${hour}:${minute}`;
}

export function formatUtcTime(
  value: TimestampInput,
  fallback = "--:--:--"
): string {
  const millis = toEpochMillis(value);

  if (millis == null) {
    return fallback;
  }

  const date = new Date(millis);

  if (Number.isNaN(date.getTime())) {
    return fallback;
  }

  const hour = pad(date.getUTCHours());
  const minute = pad(date.getUTCMinutes());
  const second = pad(date.getUTCSeconds());

  return `${hour}:${minute}:${second}`;
}

export function formatDurationShort(
  startValue: TimestampInput,
  endValue: TimestampInput
): string {
  const startMillis = toEpochMillis(startValue);
  const endMillis = toEpochMillis(endValue);

  if (startMillis == null || endMillis == null) {
    return "0s";
  }

  let remainingSeconds = Math.max(
    0,
    Math.floor((endMillis - startMillis) / 1000)
  );

  const days = Math.floor(remainingSeconds / 86_400);
  remainingSeconds %= 86_400;

  const hours = Math.floor(remainingSeconds / 3600);
  remainingSeconds %= 3600;

  const minutes = Math.floor(remainingSeconds / 60);
  const seconds = remainingSeconds % 60;

  const parts: string[] = [];

  if (days > 0) {
    parts.push(`${days}d`);
  }

  if (hours > 0) {
    parts.push(`${hours}h`);
  }

  if (minutes > 0) {
    parts.push(`${minutes}m`);
  }

  if (seconds > 0) {
    parts.push(`${seconds}s`);
  }

  if (parts.length === 0) {
    return "0s";
  }

  return parts.join(" ");
}
