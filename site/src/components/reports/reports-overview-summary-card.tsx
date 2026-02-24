"use client";

import { useExtracted } from "next-intl";
import { Subheading } from "@/components/ui/heading";
import type { ReportsSummaryEntry } from "@/lib/types/reports-list";

type ReportsOverviewSummaryCardProps = {
  title: string;
  summary: ReportsSummaryEntry;
};

const numberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

function formatNumber(value: number) {
  return numberFormatter.format(Number.isFinite(value) ? value : 0);
}

export default function ReportsOverviewSummaryCard({
  title,
  summary,
}: ReportsOverviewSummaryCardProps) {
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
            {formatNumber(summary.troopUnits)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">{t("Dead")}</dt>
          <dd className="font-semibold text-base/6 text-zinc-950 tabular-nums sm:text-sm/6 dark:text-white">
            {formatNumber(summary.dead)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">
            {t("Severely Wounded")}
          </dt>
          <dd className="font-semibold text-base/6 text-zinc-950 tabular-nums sm:text-sm/6 dark:text-white">
            {formatNumber(summary.severelyWounded)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">
            {t("Slightly Wounded")}
          </dt>
          <dd className="font-semibold text-base/6 text-zinc-950 tabular-nums sm:text-sm/6 dark:text-white">
            {formatNumber(summary.slightlyWounded)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">
            {t("Remaining")}
          </dt>
          <dd className="font-semibold text-base/6 text-zinc-950 tabular-nums sm:text-sm/6 dark:text-white">
            {formatNumber(summary.remaining)}
          </dd>
        </div>
        <div className="my-3 border-zinc-950/10 border-t dark:border-white/10" />
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">
            {t("Kill Points")}
          </dt>
          <dd className="font-semibold text-base/6 text-zinc-950 tabular-nums sm:text-sm/6 dark:text-white">
            {formatNumber(summary.killPoints)}
          </dd>
        </div>
      </dl>
    </div>
  );
}
