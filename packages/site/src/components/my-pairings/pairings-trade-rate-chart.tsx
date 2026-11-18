"use client";

import { useMemo } from "react";
import { PairingIcons } from "@/components/my-pairings/pairings-shared";
import { Link } from "@/components/ui/link";
import type { PairingAggregate } from "@/lib/pairings";
import {
  commanderName,
  formatExact,
  MIN_PAIRING_BATTLES,
  pairingKey,
  pairingName,
} from "@/lib/pairings/analytics";
import { positiveTradeRate } from "@/lib/pairings/trade-rate";

const columns =
  "grid grid-cols-[4.25rem_minmax(0,1fr)_3.5rem] gap-x-3 sm:grid-cols-[14rem_minmax(0,1fr)_4rem]";
const ticks = [0, 25, 50, 75, 100];

export function PairingsTradeRateChart({
  items,
  query,
}: {
  items: PairingAggregate[];
  query: string;
}) {
  const rows = useMemo(
    () =>
      items
        .filter((item) => item.count >= MIN_PAIRING_BATTLES)
        .flatMap((item) => {
          const interval = positiveTradeRate(item.positiveTrades, item.count);
          return interval ? [{ item, ...interval }] : [];
        })
        .toSorted((a, b) => b.rate - a.rate || b.item.count - a.item.count),
    [items]
  );

  if (!rows.length) return null;

  return (
    <div>
      <div
        className="mt-5"
        role="list"
        aria-label="Pairings ranked by positive trade rate"
        aria-description="95% Wilson intervals treat battles as independent. Shared reports, sessions, and opponents can make the uncertainty wider than shown."
      >
        {rows.map(({ item, rate, lower, upper }) => {
          const name = pairingName(item.primaryCommanderId, item.secondaryCommanderId);
          const details = `${formatExact(item.positiveTrades)} / ${formatExact(item.count)} battles · 95% interval ${lower.toFixed(1)}–${upper.toFixed(1)}%`;
          return (
            <div key={pairingKey(item)} role="listitem">
              <Link
                href={`/account/pairings/${pairingKey(item)}?${query}`}
                prefetch={false}
                className={`${columns} group items-center rounded py-2 hover:bg-white/[0.025] focus-visible:outline-2 focus-visible:outline-blue-500`}
                aria-label={`${name}: ${rate.toFixed(1)}% positive trades. ${details}. View insights.`}
              >
                <span className="flex min-w-0 items-center gap-3">
                  <PairingIcons
                    primary={item.primaryCommanderId}
                    secondary={item.secondaryCommanderId}
                  />
                  <span className="hidden min-w-0 text-xs sm:block">
                    <span className="block truncate font-medium text-zinc-200">
                      {commanderName(item.primaryCommanderId)}
                    </span>
                    <span className="block truncate text-[11px] text-zinc-500">
                      {commanderName(item.secondaryCommanderId)}
                    </span>
                  </span>
                </span>
                <span className="relative block h-8" aria-hidden="true">
                  {ticks.map((tick) => (
                    <span
                      key={tick}
                      className={`absolute inset-y-[-8px] border-l ${tick === 50 ? "border-dashed border-zinc-600" : "border-white/5"}`}
                      style={{ left: `${tick}%` }}
                    />
                  ))}
                  <span
                    className={`absolute top-1/2 left-0 h-5 -translate-y-1/2 rounded-r-sm transition-opacity group-hover:opacity-100 ${rate >= 50 ? "bg-emerald-500/65" : "bg-blue-500/55"}`}
                    style={{ width: `${rate}%` }}
                  />
                  <span
                    className="absolute top-1/2 h-0 border-t-2 border-zinc-100"
                    style={{ left: `${lower}%`, width: `${upper - lower}%` }}
                  >
                    <span className="absolute -top-1.5 left-0 h-3 border-l-2 border-zinc-100" />
                    <span className="absolute -top-1.5 right-0 h-3 border-r-2 border-zinc-100" />
                  </span>
                  <span
                    className="absolute top-1/2 size-1.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-white"
                    style={{ left: `${rate}%` }}
                  />
                </span>
                <span className="text-right text-sm font-medium tabular-nums text-zinc-100">
                  {rate.toFixed(1)}%
                </span>
              </Link>
            </div>
          );
        })}
      </div>
      <div className={`${columns} mt-2`} aria-hidden="true">
        <span />
        <span className="relative block h-4 text-[10px] tabular-nums text-zinc-500">
          {ticks.map((tick) => (
            <span key={tick} className="absolute -translate-x-1/2" style={{ left: `${tick}%` }}>
              {tick}%
            </span>
          ))}
        </span>
      </div>
    </div>
  );
}
