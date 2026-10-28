export const scoreFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 2,
  minimumFractionDigits: 2,
});

const dateFormatter = new Intl.DateTimeFormat("en-US", {
  day: "2-digit",
  hour: "2-digit",
  hourCycle: "h23",
  minute: "2-digit",
  month: "2-digit",
  timeZone: "UTC",
});

export function clampScore(value: number): number {
  return Number.isFinite(value) ? Math.min(10, Math.max(0, value)) : 0;
}

export function formatRefreshedAt(value: string): string {
  return `${dateFormatter.format(new Date(value)).replace(",", "")} UTC`;
}
