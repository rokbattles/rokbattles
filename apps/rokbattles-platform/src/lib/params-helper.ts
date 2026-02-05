export function toInt(v: string | string[] | undefined, fallback: number) {
  const s = Array.isArray(v) ? v[0] : v;
  const n = Number.parseInt(s ?? "", 10);
  return Number.isFinite(n) ? n : fallback;
}
