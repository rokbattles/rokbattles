export const scoreFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 2,
  minimumFractionDigits: 2,
});

export function clampScore(value: number): number {
  return Number.isFinite(value) ? Math.min(10, Math.max(0, value)) : 0;
}

export { formatCompactUtcDateTime as formatRefreshedAt } from "@/lib/datetime";
