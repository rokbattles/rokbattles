type NumberInput = number | null | undefined;

export function formatNumberOrFallback(
  value: NumberInput,
  fallback = "-"
): string {
  if (value == null || !Number.isFinite(value)) {
    return fallback;
  }

  return value.toLocaleString();
}

export function formatSignedNumberOrFallback(
  value: NumberInput,
  fallback = "-"
): string {
  if (value == null || !Number.isFinite(value)) {
    return fallback;
  }

  if (value > 0) {
    return `+${value.toLocaleString()}`;
  }

  return value.toLocaleString();
}

export function formatAbbreviatedNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "-";
  }

  const absoluteValue = Math.abs(value);

  if (absoluteValue >= 1_000_000_000_000) {
    return `${(value / 1_000_000_000_000).toFixed(2)}T`;
  }

  if (absoluteValue >= 1_000_000_000) {
    return `${(value / 1_000_000_000).toFixed(2)}B`;
  }

  if (absoluteValue >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(2)}M`;
  }

  return Math.round(value).toLocaleString();
}
