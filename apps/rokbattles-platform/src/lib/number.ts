export function coerceNumber(value: unknown, fallback = 0): number {
  const numeric = typeof value === "bigint" ? Number(value) : Number(value);
  return Number.isFinite(numeric) ? numeric : fallback;
}
