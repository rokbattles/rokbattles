"use client";

const numberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

type LootTimelineTooltipClientProps = {
  active?: boolean;
  label?: string | number;
  payload?: Array<{ value?: number | string }>;
  killsLabel: string;
};

export function LootTimelineTooltipClient({
  active,
  label,
  payload,
  killsLabel,
}: LootTimelineTooltipClientProps) {
  if (!active || !payload || payload.length === 0) {
    return null;
  }

  const rawValue = payload[0]?.value;
  const numericValue = Number(rawValue);
  const value = Number.isFinite(numericValue) ? numericValue : 0;

  return (
    <div className="rounded-lg border border-zinc-200 bg-white px-3 py-2 shadow-sm dark:border-zinc-700 dark:bg-zinc-900">
      <div className="text-xs text-zinc-500 dark:text-zinc-400">{String(label ?? "")}</div>
      <div className="mt-1 text-sm font-semibold text-zinc-950 dark:text-zinc-100">
        {numberFormatter.format(value)} {killsLabel}
      </div>
    </div>
  );
}
