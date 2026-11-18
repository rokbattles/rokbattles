"use client";

import { useEffect, useMemo, useState } from "react";
import { PairingPositionMarkers } from "@/components/my-pairings/pairing-position-markers";
import { PairingIcons } from "@/components/my-pairings/pairings-shared";
import { Link } from "@/components/ui/link";
import type { PairingAggregate } from "@/lib/pairings";
import {
  formatCompact,
  formatExact,
  formatTrade,
  MIN_PAIRING_BATTLES,
  pairingKey,
  pairingName,
} from "@/lib/pairings/analytics";
import { efficiencyPoints } from "@/lib/pairings/efficiency";

export function PairingsEfficiencyChart({
  items,
  query,
}: {
  items: PairingAggregate[];
  query: string;
}) {
  const [plot, setPlot] = useState<HTMLDivElement | null>(null);
  const [width, setWidth] = useState(1000);
  const [hovered, setHovered] = useState<string | null>(null);
  const [focused, setFocused] = useState<string | null>(null);

  useEffect(() => {
    if (!plot) return;

    const observer = new ResizeObserver(([entry]) => {
      if (entry.contentRect.width > 0 && entry.contentRect.height > 0) {
        setWidth(entry.contentRect.width);
      }
    });

    observer.observe(plot);
    return () => observer.disconnect();
  }, [plot]);

  const eligible = useMemo(
    () =>
      items.filter(
        (item) =>
          item.count >= MIN_PAIRING_BATTLES &&
          item.totals.killScore + item.totals.enemyKillScore > 0
      ),
    [items]
  );
  const {
    points,
    pivot,
    height: plotHeight,
  } = useMemo(() => efficiencyPoints(eligible, width, width < 500 ? 300 : 356), [eligible, width]);

  return (
    <div className="relative mt-6" style={{ height: plotHeight + 84 }}>
      <div ref={setPlot} className="absolute top-7 right-5 bottom-14 left-16">
        <svg
          className="absolute inset-0 size-full overflow-visible"
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          <rect width="50" height="50" fill="#34d399" fillOpacity="0.045" />
          <rect x="50" y="50" width="50" height="50" fill="#fb7185" fillOpacity="0.045" />
          {[0, 25, 75, 100].map((value) => (
            <g key={value}>
              <line
                x1="0"
                x2="100"
                y1={value}
                y2={value}
                stroke="#fff"
                strokeOpacity="0.05"
                vectorEffect="non-scaling-stroke"
              />
              <line
                x1={value}
                x2={value}
                y1="0"
                y2="100"
                stroke="#fff"
                strokeOpacity="0.035"
                vectorEffect="non-scaling-stroke"
              />
            </g>
          ))}
          <line
            x1="50"
            x2="50"
            y1="0"
            y2="100"
            stroke="#71717a"
            strokeDasharray="4 5"
            vectorEffect="non-scaling-stroke"
          />
          <line
            x1="0"
            x2="100"
            y1="50"
            y2="50"
            stroke="#71717a"
            strokeDasharray="4 5"
            vectorEffect="non-scaling-stroke"
          />
        </svg>
        <PairingPositionMarkers
          points={points}
          width={width}
          height={plotHeight}
          activeKey={hovered ?? focused}
        />
        <span className="absolute -top-6 left-0 text-[10px] font-medium text-emerald-300">
          Efficient
        </span>
        <span className="absolute -top-6 right-0 text-[10px] text-zinc-500">Costly wins</span>
        <span className="pointer-events-none absolute bottom-2 left-2 text-[10px] text-zinc-500">
          Lower trade
        </span>
        <span className="pointer-events-none absolute right-2 bottom-2 text-[10px] text-rose-300">
          Costly losses
        </span>
        {points.map(({ item, x, y }) => (
          <Link
            key={pairingKey(item)}
            href={`/account/pairings/${pairingKey(item)}?${query}`}
            prefetch={false}
            aria-label={`${pairingName(item.primaryCommanderId, item.secondaryCommanderId)}: ${formatTrade(item.totals)} trade, ${formatExact(item.count)} battles. View insights.`}
            style={{ left: `${(x / width) * 100}%`, top: `${(y / plotHeight) * 100}%` }}
            className="absolute z-10 -translate-x-1/2 -translate-y-1/2 rounded p-0.5 focus-visible:outline-2 focus-visible:outline-blue-500"
            onMouseEnter={() => setHovered(pairingKey(item))}
            onMouseLeave={() => setHovered(null)}
            onFocus={() => setFocused(pairingKey(item))}
            onBlur={() => setFocused(null)}
          >
            <PairingIcons
              primary={item.primaryCommanderId}
              secondary={item.secondaryCommanderId}
              chart
              compact={width < 500}
            />
          </Link>
        ))}
        {[0, 25, 50, 75, 100].map((value, index) => (
          <span
            key={value}
            className="absolute -left-2 -translate-x-full -translate-y-1/2 text-[10px] tabular-nums text-zinc-500"
            style={{ top: `${value}%` }}
          >
            {["∞", "300%", "100%", "33%", "0%"][index]}
          </span>
        ))}
        {[0, 25, 50, 75, 100].map((value, index) => (
          <span
            key={value}
            style={{ left: `${value}%` }}
            className="absolute -bottom-6 -translate-x-1/2 text-[10px] tabular-nums text-zinc-500"
          >
            {index === 4 ? "∞" : formatCompact([0, pivot / 3, pivot, pivot * 3][index])}
          </span>
        ))}
      </div>
      <span
        className="absolute top-1/2 left-0 -translate-y-1/2 rotate-180 text-xs text-zinc-400"
        style={{ writingMode: "vertical-rl" }}
      >
        Trade %
      </span>
      <span className="absolute right-5 bottom-0 left-16 text-center text-xs text-zinc-400">
        Casualties / battle
      </span>
    </div>
  );
}
