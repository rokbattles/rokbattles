import { getCommanderName } from "@/lib/commander";
import type { PairingAggregate, PairingTotals } from "@/lib/pairings";

const compact = new Intl.NumberFormat("en-US", { notation: "compact", maximumFractionDigits: 1 });
const exact = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });

export const formatCompact = (value: number) => compact.format(value);
export const formatExact = (value: number) => exact.format(value);

export const netKp = (totals: PairingTotals) => totals.killScore - totals.enemyKillScore;
export const casualties = (totals: PairingTotals) => totals.severelyWounded + totals.deaths;
export const tradeRatio = (totals: Pick<PairingTotals, "killScore" | "enemyKillScore">) =>
  totals.enemyKillScore > 0 ? (totals.killScore / totals.enemyKillScore) * 100 : null;

export function formatTrade(totals: Pick<PairingTotals, "killScore" | "enemyKillScore">) {
  const trade = tradeRatio(totals);
  return trade === null ? (totals.killScore > 0 ? "No KP lost" : "—") : `${trade.toFixed(1)}%`;
}

export const pairingKey = (pairing: PairingAggregate) =>
  `${pairing.primaryCommanderId}-${pairing.secondaryCommanderId}`;
export const commanderName = (id: number) =>
  id > 0 ? (getCommanderName(id) ?? `Commander ${id}`) : "No secondary";
export const pairingName = (primary: number, secondary: number) =>
  `${commanderName(primary)}${secondary > 0 ? ` / ${commanderName(secondary)}` : ""}`;

export const MIN_PAIRING_BATTLES = 100;
