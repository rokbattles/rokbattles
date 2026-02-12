export function formatTradePercentage(value: number): string {
  return `${Math.round(value).toLocaleString()}%`;
}
