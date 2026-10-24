"use client";

import { Check } from "lucide-react";
import { useExtracted, useFormatter } from "next-intl";
import type { ReactElement } from "react";
import { LootIcon } from "@/components/account-loot/loot-icon";
import type { ResourceKey } from "@/lib/resources/catalog";
import type { ResourceBreakdownRow } from "@/lib/resources/rows";

type ResourcesSummaryClientProps = {
  rows: ResourceBreakdownRow[];
  selected: ResourceKey[];
  days: number;
  onToggle: (key: ResourceKey) => void;
};

export function ResourcesSummaryClient({
  rows,
  selected,
  days,
  onToggle,
}: ResourcesSummaryClientProps): ReactElement {
  const t = useExtracted();
  const intl = useFormatter();

  return (
    <div className="grid grid-cols-2 gap-3 lg:grid-cols-3 xl:grid-cols-6">
      {rows.map((row) => {
        const active = selected.includes(row.key);
        const average = row.total / Math.max(days, 1);

        return (
          <button
            key={row.key}
            type="button"
            aria-pressed={active}
            aria-label={t("Show {resource}", { resource: row.name })}
            onClick={() => onToggle(row.key)}
            className={`relative rounded-md border p-4 text-left transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 ${
              active ? "border-zinc-600 bg-zinc-900" : "text-zinc-500 border-zinc-800 bg-zinc-950"
            }`}
          >
            <div className="mb-4 flex items-center justify-between gap-2">
              <LootIcon spriteUrls={row.spriteUrls} />
              <span
                className={`flex size-5 items-center justify-center rounded-full border ${active ? "border-white bg-white text-zinc-900" : "border-zinc-700"}`}
              >
                {active ? <Check aria-hidden="true" className="size-3" /> : null}
              </span>
            </div>
            <div className="flex items-center gap-2 text-sm text-zinc-400">
              <span
                className="size-2 shrink-0 rounded-full"
                style={{ backgroundColor: row.color }}
              />
              {row.name}
            </div>
            <div
              className="mt-1 text-2xl font-semibold tabular-nums tracking-tight text-white"
              title={intl.number(row.total, { maximumFractionDigits: 0 })}
            >
              {intl.number(row.total, { notation: "compact", maximumFractionDigits: 1 })}
            </div>
            <p
              className="mt-1 text-xs text-zinc-400"
              title={intl.number(average, { maximumFractionDigits: 0 })}
            >
              {t("{amount} / day", {
                amount: intl.number(average, { notation: "compact", maximumFractionDigits: 1 }),
              })}
            </p>
          </button>
        );
      })}
    </div>
  );
}
