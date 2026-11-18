"use client";

import { pairingKey } from "@/lib/pairings/analytics";
import { type PairingPlotPoint, pairingConnector } from "@/lib/pairings/efficiency";

export function PairingPositionMarkers({
  points,
  width,
  height,
  activeKey,
}: {
  points: PairingPlotPoint[];
  width: number;
  height: number;
  activeKey: string | null;
}) {
  const ordered = activeKey
    ? points.toSorted(
        (a, b) =>
          Number(pairingKey(a.item) === activeKey) - Number(pairingKey(b.item) === activeKey)
      )
    : points;

  return (
    <svg
      className="pointer-events-none absolute inset-0 size-full overflow-visible"
      viewBox={`0 0 ${width} ${height}`}
      aria-hidden="true"
    >
      {ordered.map((point) => {
        const { x1, y1, x2, y2 } = pairingConnector(point);
        const active = pairingKey(point.item) === activeKey;
        return (
          <g key={pairingKey(point.item)}>
            <line
              x1={x1}
              y1={y1}
              x2={x2}
              y2={y2}
              fill="none"
              stroke={active ? "#60a5fa" : "#a1a1aa"}
              strokeOpacity={active ? 1 : 0.7}
              strokeWidth={active ? 1.5 : 1}
              strokeLinecap="round"
            />
            <circle
              cx={point.anchorX}
              cy={point.anchorY}
              r={active ? 3.5 : 3}
              fill={active ? "#60a5fa" : "#d4d4d8"}
              stroke="#18181b"
              strokeWidth="1"
            />
          </g>
        );
      })}
    </svg>
  );
}
