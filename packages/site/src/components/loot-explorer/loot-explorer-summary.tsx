import { cn } from "cnfast";
import { useExtracted } from "next-intl";
import { formatGeneratedAt, formatNumber } from "@/lib/loot-explorer/format";

export type LootExplorerSummaryItem = {
  label: string;
  value: string | number;
  description?: string;
};

export function LootExplorerSummary({
  generatedAt,
  items,
  maxColumns = 5,
}: {
  generatedAt?: string;
  items: LootExplorerSummaryItem[];
  maxColumns?: 4 | 5;
}) {
  const t = useExtracted();
  const summaryItems = generatedAt
    ? [
        ...items,
        {
          label: t("Last updated"),
          value: formatGeneratedAt(generatedAt),
        },
      ]
    : items;

  return (
    <div
      className={cn(
        "grid grid-cols-2 gap-6",
        maxColumns === 4 ? "md:grid-cols-4" : "md:grid-cols-3 xl:grid-cols-5"
      )}
    >
      {summaryItems.map((item) => (
        <div
          key={item.label}
          className="space-y-3 border-zinc-200/60 border-b pb-4 dark:border-white/10"
        >
          <div className="space-y-1">
            <div className="font-semibold text-sm text-zinc-950 dark:text-white">{item.label}</div>
            {item.description ? (
              <p className="text-sm text-zinc-600 dark:text-zinc-400">{item.description}</p>
            ) : null}
          </div>
          <div className="mt-4 font-semibold text-2xl/8 text-zinc-950 dark:text-white">
            {typeof item.value === "number" ? formatNumber(item.value) : item.value}
          </div>
        </div>
      ))}
    </div>
  );
}
