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
    <div className="rounded border p-4 border-white/10 bg-white/5">
      <Subheading className="mb-3">{title}</Subheading>
      <dl className="space-y-2">
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 sm:text-sm/6 text-zinc-400">{t("Troop Units")}</dt>
          <dd className="font-semibold text-base/6 tabular-nums sm:text-sm/6 text-white">
            {formatNumber(summary.troopUnits)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 sm:text-sm/6 text-zinc-400">{t("Dead")}</dt>
          <dd className="font-semibold text-base/6 tabular-nums sm:text-sm/6 text-white">
            {formatNumber(summary.dead)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 sm:text-sm/6 text-zinc-400">{t("Severely Wounded")}</dt>
          <dd className="font-semibold text-base/6 tabular-nums sm:text-sm/6 text-white">
            {formatNumber(summary.severelyWounded)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 sm:text-sm/6 text-zinc-400">{t("Slightly Wounded")}</dt>
          <dd className="font-semibold text-base/6 tabular-nums sm:text-sm/6 text-white">
            {formatNumber(summary.slightlyWounded)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 sm:text-sm/6 text-zinc-400">{t("Remaining")}</dt>
          <dd className="font-semibold text-base/6 tabular-nums sm:text-sm/6 text-white">
            {formatNumber(summary.remaining)}
          </dd>
        </div>
        <div className="my-3 border-t border-white/10" />
        <div className="flex items-center justify-between gap-4">
          <dt className="text-base/6 sm:text-sm/6 text-zinc-400">{t("Kill Points")}</dt>
          <dd className="font-semibold text-base/6 tabular-nums sm:text-sm/6 text-white">
            {formatNumber(summary.killPoints)}
          </dd>
        </div>
      </dl>
    </div>
  );
}
