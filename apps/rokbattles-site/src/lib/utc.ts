export function formatUTCShort(ts?: number): string | undefined {
  if (!ts || ts <= 0) return undefined;
  try {
    const d = new Date(ts * 1000);
    const mm = String(d.getUTCMonth() + 1).padStart(2, "0");
    const dd = String(d.getUTCDate()).padStart(2, "0");
    const hh = String(d.getUTCHours()).padStart(2, "0");
    const min = String(d.getUTCMinutes()).padStart(2, "0");
    return `UTC ${mm}/${dd} ${hh}:${min}`;
  } catch {
    return undefined;
  }
}
