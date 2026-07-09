export const numberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

export const percentFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 2,
  style: "percent",
});

export const decimalFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 2,
});

export function formatNumber(value: number): string {
  return Number.isFinite(value) ? numberFormatter.format(value) : "0";
}

export function formatPercent(value: number): string {
  return Number.isFinite(value) ? percentFormatter.format(value) : "0%";
}

export function formatQuantity(quantity: { min: number; max: number }): string {
  if (quantity.min === quantity.max) {
    return formatNumber(quantity.min);
  }

  return `${formatNumber(quantity.min)}-${formatNumber(quantity.max)}`;
}

export function formatRange(
  range: { min: number | null; max: number | null },
  suffix = "",
  fallback = "n/a"
): string {
  if (range.min == null && range.max == null) {
    return fallback;
  }

  if (range.min === range.max) {
    return `${decimalFormatter.format(range.min ?? 0)}${suffix}`;
  }

  return `${decimalFormatter.format(range.min ?? 0)}-${decimalFormatter.format(range.max ?? 0)}${suffix}`;
}

export function formatGeneratedAt(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  const month = (date.getUTCMonth() + 1).toString().padStart(2, "0");
  const day = date.getUTCDate().toString().padStart(2, "0");
  const hour = date.getUTCHours().toString().padStart(2, "0");
  const minute = date.getUTCMinutes().toString().padStart(2, "0");

  return `${month}/${day} ${hour}:${minute} UTC`;
}
