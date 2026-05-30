"use client";

type TooltipPayloadEntry = {
  dataKey?: string;
  value?: number | string;
  color?: string;
};

type ResourcesTimelineTooltipClientProps = {
  active?: boolean;
  label?: string | number;
  payload?: TooltipPayloadEntry[];
  labels: Record<string, string>;
  order: string[];
};

const numberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

export function ResourcesTimelineTooltipClient({
  active,
  label,
  payload,
  labels,
  order,
}: ResourcesTimelineTooltipClientProps) {
  if (!active || !payload || payload.length === 0) {
    return null;
  }

  const orderMap = new Map(order.map((key, index) => [key, index]));
  const rows = payload
    .filter((entry) => entry.dataKey && orderMap.has(entry.dataKey))
    .sort(
      (left, right) =>
        (orderMap.get(left.dataKey ?? "") ?? Number.MAX_SAFE_INTEGER) -
        (orderMap.get(right.dataKey ?? "") ?? Number.MAX_SAFE_INTEGER)
    );

  if (rows.length === 0) {
    return null;
  }

  return (
    <div className="rounded-lg border border-zinc-200 bg-white px-3 py-2 shadow-sm dark:border-zinc-700 dark:bg-zinc-900">
      <div className="text-xs text-zinc-500 dark:text-zinc-400">{String(label ?? "")}</div>
      <div className="mt-2 space-y-1">
        {rows.map((row) => {
          const key = row.dataKey as string;
          const labelText = labels[key];
          if (!labelText) {
            return null;
          }

          const numericValue = Number(row.value);
          const value = Number.isFinite(numericValue) ? numericValue : 0;

          return (
            <div
              key={key}
              className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-200"
            >
              <span
                aria-hidden="true"
                className="size-2 rounded-full"
                style={{ backgroundColor: row.color ?? "#71717a" }}
              />
              <span className="min-w-20">{labelText}</span>
              <span className="ml-auto tabular-nums text-zinc-900 dark:text-zinc-100">
                {numberFormatter.format(value)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
