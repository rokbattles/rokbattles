import type { TooltipProps } from "recharts";

type DuelSummaryTooltipProps = TooltipProps<number, string> & {
  formatter: Intl.NumberFormat;
};

export function DuelSummaryTooltip({ active, payload, label, formatter }: DuelSummaryTooltipProps) {
  if (!active || !payload || payload.length === 0 || !label) {
    return null;
  }

  const entries = payload
    .filter((entry) => typeof entry.dataKey === "string")
    .map((entry) => ({
      key: entry.dataKey as string,
      value: Number(entry.value ?? 0),
    }));

  return (
    <div className="rounded-lg border border-zinc-950/10 bg-white px-4 py-3 text-xs shadow-lg dark:border-white/10 dark:bg-zinc-900">
      <div className="font-semibold text-zinc-700 dark:text-zinc-100">{label}</div>
      <div className="mt-3 space-y-1.5">
        {entries.map((entry) => {
          const descriptor =
            entry.key === "sender"
              ? { label: "Sender", color: "#3b82f6" }
              : entry.key === "opponent"
                ? { label: "Opponent", color: "#f87171" }
                : null;
          if (!descriptor) {
            return null;
          }
          return (
            <div key={descriptor.label} className="flex items-center gap-3">
              <span
                className="inline-block size-2 rounded-full"
                style={{ backgroundColor: descriptor.color }}
                aria-hidden="true"
              />
              <span className="flex-1 text-zinc-600 dark:text-zinc-300">{descriptor.label}</span>
              <span className="font-mono text-zinc-800 dark:text-white">
                {formatter.format(entry.value)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
