"use client";

import { useTranslations } from "next-intl";

type ReportBattleSummaryTooltipProps = {
  active?: boolean;
  payload?: unknown;
  label?: string | number;
};

const numberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

export function ReportBattleSummaryTooltip({
  active,
  payload,
  label,
}: ReportBattleSummaryTooltipProps) {
  const t = useTranslations("common");
  if (
    !(active && Array.isArray(payload)) ||
    payload.length === 0 ||
    label == null
  ) {
    return null;
  }

  const entries = payload
    .filter((entry) => entry && typeof entry === "object" && "dataKey" in entry)
    .map((entry) => {
      const record = entry as { dataKey?: unknown; value?: unknown };
      return {
        key: record.dataKey,
        value: Number(record.value ?? 0),
      };
    })
    .filter(
      (entry): entry is { key: string; value: number } =>
        typeof entry.key === "string"
    );

  return (
    <div className="rounded-lg border border-zinc-950/10 bg-white px-4 py-3 text-xs shadow-lg dark:border-white/10 dark:bg-zinc-900">
      <div className="font-semibold text-zinc-700 dark:text-zinc-100">
        {String(label)}
      </div>
      <div className="mt-3 space-y-1.5">
        {entries.map((entry) => {
          const descriptor =
            entry.key === "self"
              ? { label: t("labels.sender"), color: "#3b82f6" }
              : entry.key === "enemy"
                ? { label: t("labels.opponent"), color: "#f87171" }
                : null;
          if (!descriptor) {
            return null;
          }
          return (
            <div className="flex items-center gap-3" key={descriptor.label}>
              <span
                aria-hidden="true"
                className="inline-block size-2 rounded-full"
                style={{ backgroundColor: descriptor.color }}
              />
              <span className="flex-1 text-zinc-600 dark:text-zinc-300">
                {descriptor.label}
              </span>
              <span className="font-mono text-zinc-800 dark:text-white">
                {numberFormatter.format(entry.value)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
