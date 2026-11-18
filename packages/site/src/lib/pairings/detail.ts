import type { PairingStats } from "@/lib/pairings";

export function formatDetailTrade(stats: PairingStats) {
  return stats.tradePercent === null
    ? stats.kpGain > 0
      ? "No KP lost"
      : "—"
    : `${stats.tradePercent.toFixed(1)}%`;
}
