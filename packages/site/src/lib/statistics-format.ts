export const decimalFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 2,
  minimumFractionDigits: 2,
});

export function formatPerSecond(value: number): string {
  return `${decimalFormatter.format(Number.isFinite(value) ? value : 0)}/s`;
}
