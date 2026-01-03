"use client";

import { useTranslations } from "next-intl";
import { getArmamentInfo } from "@/hooks/useArmamentName";
import type { LoadoutSnapshot } from "@/hooks/usePairings";

type LoadoutArmamentListProps = {
  armaments: LoadoutSnapshot["armaments"];
};

export function LoadoutArmamentList({ armaments }: LoadoutArmamentListProps) {
  const tReport = useTranslations("report");

  if (armaments.length === 0) {
    return <div className="min-h-5" />;
  }

  return (
    <div className="space-y-1 text-xs text-zinc-600 dark:text-zinc-300">
      {armaments.map((buff) => {
        const fallbackId = typeof buff.id === "number" ? buff.id : "?";
        const name =
          getArmamentInfo(buff.id ?? null)?.name ??
          tReport("armament.fallback", { id: fallbackId });
        const valueLabel =
          typeof buff.value === "number" ? `${(buff.value * 100).toFixed(2)}%` : null;

        return (
          <div key={`${buff.id}-${buff.value ?? "none"}`} className="flex items-center gap-2">
            <span className="min-w-0 flex-1 truncate">{name}</span>
            {valueLabel ? (
              <span className="tabular-nums text-zinc-500 dark:text-zinc-400">{valueLabel}</span>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}
