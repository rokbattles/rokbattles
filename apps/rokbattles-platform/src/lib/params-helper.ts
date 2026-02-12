export function toStr(v: string | string[] | undefined) {
  if (!v) {
    return undefined;
  }

  return Array.isArray(v) ? v[0] : v;
}

export function toInt(v: string | string[] | undefined, fallback: number) {
  const s = toStr(v);
  const n = Number.parseInt(s ?? "", 10);
  return Number.isFinite(n) ? n : fallback;
}
