"use client";

import { useFormatter } from "next-intl";
import type { ReactElement } from "react";
import type { ResourceBreakdownRow } from "@/lib/resources/rows";

type TooltipPayloadEntry = {
  dataKey?: string;
  value?: number | string;
  color?: string;
};

type ResourcesTimelineTooltipClientProps = {
  active?: boolean;
  label?: string | number;
  payload?: TooltipPayloadEntry[];
  rows: ResourceBreakdownRow[];
};

export function ResourcesTimelineTooltipClient({
  active,
  label,
  payload,
  rows,
}: ResourcesTimelineTooltipClientProps): ReactElement | null {
  const intl = useFormatter();

  if (!active || !payload?.length) {
    return null;
  }

  const entries = rows.flatMap((row) => {
    const entry = payload.find((item) => item.dataKey === row.key);

    if (!entry || !row.name) {
      return [];
    }

    const value = Number(entry.value);

    return [
      {
        key: row.key,
        name: row.name,
        color: entry.color ?? "#71717a",
        value: Number.isFinite(value) ? value : 0,
      },
    ];
  });

  if (entries.length === 0) {
    return null;
  }

  return (
    <div className="rounded-lg border border-zinc-200 bg-white px-3 py-2 shadow-sm dark:border-zinc-700 dark:bg-zinc-900">
      <div className="text-xs text-zinc-500 dark:text-zinc-400">
        {intl.dateTime(new Date(`${label}T00:00:00Z`), {
          month: "long",
          day: "numeric",
          year: "numeric",
          timeZone: "UTC",
        })}
      </div>
      <div className="mt-2 space-y-1">
        {entries.map((entry) => (
          <div
            key={entry.key}
            className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-200"
          >
            <span
              aria-hidden="true"
              className="size-2 rounded-full"
              style={{ backgroundColor: entry.color }}
            />
            <span className="min-w-20">{entry.name}</span>
            <span className="ml-auto tabular-nums text-zinc-900 dark:text-zinc-100">
              {intl.number(entry.value, { maximumFractionDigits: 0 })}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
