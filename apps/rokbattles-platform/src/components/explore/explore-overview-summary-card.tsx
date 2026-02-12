"use client";

import { useExtracted } from "next-intl";
import { Subheading } from "@/components/ui/heading";
import {
  formatNumberOrFallback,
  formatSignedNumberOrFallback,
} from "@/lib/numbers";
import type { ExploreBattleSummaryEntry } from "@/lib/types/explore-battle-reports";

export default function ExploreOverviewSummaryCard({
  title,
  summary,
}: {
  title: string;
  summary: ExploreBattleSummaryEntry;
}) {
  const t = useExtracted();

  return (
    <div className="rounded border border-zinc-950/10 bg-zinc-50/60 p-4 dark:border-white/10 dark:bg-white/5">
      <Subheading className="mb-3">{title}</Subheading>
      <dl className="space-y-2">
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">
            {t("Troop Units")}
          </dt>
          <dd className="font-semibold text-base/6 text-zinc-950 tabular-nums sm:text-sm/6 dark:text-white">
            {formatNumberOrFallback(summary.troopUnits)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">
            {t("Dead")}
          </dt>
          <dd className="font-semibold text-base/6 text-zinc-950 tabular-nums sm:text-sm/6 dark:text-white">
            {formatNumberOrFallback(summary.dead)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">
            {t("Severely Wounded")}
          </dt>
          <dd className="font-semibold text-base/6 text-zinc-950 tabular-nums sm:text-sm/6 dark:text-white">
            {formatNumberOrFallback(summary.severelyWounded)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">
            {t("Slightly Wounded")}
          </dt>
          <dd className="font-semibold text-base/6 text-zinc-950 tabular-nums sm:text-sm/6 dark:text-white">
            {formatNumberOrFallback(summary.slightlyWounded)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">
            {t("Remaining")}
          </dt>
          <dd className="font-semibold text-base/6 text-zinc-950 tabular-nums sm:text-sm/6 dark:text-white">
            {formatNumberOrFallback(summary.remaining)}
          </dd>
        </div>
        <div className="my-3 border-zinc-950/10 border-t dark:border-white/10" />
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">
            {t("Kill Points")}
          </dt>
          <dd className="font-semibold text-base/6 text-zinc-950 tabular-nums sm:text-sm/6 dark:text-white">
            {formatSignedNumberOrFallback(summary.killPoints)}
          </dd>
        </div>
      </dl>
    </div>
  );
}
