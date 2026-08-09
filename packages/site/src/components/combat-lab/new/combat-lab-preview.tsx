"use client";

import { ArrowLeftIcon, InformationCircleIcon } from "@heroicons/react/16/solid";
import dynamic from "next/dynamic";
import Link from "next/link";
import { useExtracted, useLocale } from "next-intl";
import { type CSSProperties, useState } from "react";
import { CombatLabPreviewDrastc } from "@/components/combat-lab/new/combat-lab-preview-drastc";
import { CombatLabPreviewEmptyState } from "@/components/combat-lab/new/combat-lab-preview-empty-state";
import { CombatLabPreviewLoadouts } from "@/components/combat-lab/new/combat-lab-preview-loadouts";
import { CommanderIcon } from "@/components/commander-icon";
import { Heading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { formatRefreshedAt } from "@/lib/combat-lab/format";
import {
  type CombatLabPreviewData,
  type CombatLabPreviewRangeKey,
  type CombatLabPreviewScenarioKey,
  type CombatLabPreviewSummary,
  combatLabPreviewRangeKeys,
  combatLabPreviewScenarioKeys,
  type CombatLabPreviewDrastc as DrastcData,
} from "@/lib/combat-lab/preview-types";

const CombatLabPreviewCharts = dynamic(
  () =>
    import("@/components/combat-lab/new/combat-lab-preview-charts").then(
      (module) => module.CombatLabPreviewCharts
    ),
  { loading: () => <ChartSkeleton />, ssr: false }
);

const drastcCategoryKeys = [
  "damage",
  "rage",
  "assist",
  "sustainability",
  "trade",
  "consistency",
] as const;
const skeletonChartBarHeights = [35, 62, 48, 76, 58, 84, 68] as const;

export function CombatLabPreview({ data }: { data: CombatLabPreviewData }) {
  const t = useExtracted();
  const locale = useLocale();
  const rangeLabels: Record<CombatLabPreviewRangeKey, string> = {
    "1y": t("1 year"),
    "6m": t("6 months"),
    "1m": t("1 month"),
    "7d": t("7 days"),
  };
  const scenarioLabels: Record<CombatLabPreviewScenarioKey, string> = {
    all: t("All"),
    openField: t("Open field"),
    swarming: t("Swarming"),
    rally: t("Rally"),
    garrison: t("Garrison"),
  };
  const compactFormatter = new Intl.NumberFormat(locale, {
    notation: "compact",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
  const numberFormatter = new Intl.NumberFormat(locale);
  const decimalFormatter = new Intl.NumberFormat(locale, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
  const [rangeKey, setRangeKey] = useState<CombatLabPreviewRangeKey>("1y");
  const [scenarioKey, setScenarioKey] = useState<CombatLabPreviewScenarioKey>("openField");

  const selected = data.ranges?.[rangeKey]?.scenarios?.[scenarioKey];
  const summary = normalizeSummary(selected?.summary);
  const hasCombatSummary = summary.battles > 0;
  const hasSummaryMetric = (key: keyof CombatLabPreviewSummary) =>
    isFiniteNumber(selected?.summary?.[key]);
  const trends = Array.isArray(selected?.trends) ? selected.trends : [];
  const formationUsage = Array.isArray(selected?.formationUsage) ? selected.formationUsage : [];
  const drastc = isCompleteDrastc(data.drastc) ? data.drastc : null;
  const updated = formatRefreshedAt(new Date(data.generatedAtMs).toISOString(), locale);

  return (
    <div className="min-h-dvh text-zinc-950 dark:text-white">
      <header className="relative -mx-6 -mt-6 overflow-hidden border-zinc-950/10 border-b bg-zinc-950 text-white lg:-mx-10 lg:-mt-10 lg:rounded-t-lg dark:border-white/10">
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_80%_0%,rgba(37,99,235,.28),transparent_42%),radial-gradient(circle_at_20%_100%,rgba(124,58,237,.18),transparent_38%)]" />
        <div className="relative mx-auto max-w-7xl px-4 py-7 sm:px-6 sm:py-10 lg:px-8">
          <Link
            href="/combat-lab/rankings"
            className="inline-flex items-center gap-1.5 font-medium text-sm text-zinc-400 transition hover:text-white"
          >
            <ArrowLeftIcon className="size-4" /> {t("Back to Combat Lab")}
          </Link>
          <div className="mt-8 flex items-center gap-3">
            <div className="flex">
              <CommanderIcon
                id={data.pairing.primaryCommanderId}
                alt={data.pairing.primaryCommanderName}
                awakened
                className="size-14 sm:size-16"
                loading="eager"
                sizes="64px"
              />
              <CommanderIcon
                id={data.pairing.secondaryCommanderId}
                alt={data.pairing.secondaryCommanderName}
                awakened
                className="-ml-3 size-14 sm:size-16"
                loading="eager"
                sizes="64px"
              />
            </div>
            <div>
              <h1 className="font-semibold text-2xl tracking-tight sm:text-4xl">
                {data.pairing.primaryCommanderName} <span className="text-zinc-500">/</span>{" "}
                {data.pairing.secondaryCommanderName}
              </h1>
              <Text className="mt-1 !text-sm/5 !text-zinc-400 sm:!text-base/6">
                {t("Updated at {date}", { date: updated })}
              </Text>
            </div>
          </div>
        </div>
      </header>

      <div className="z-20 -mx-6 border-zinc-950/10 border-b bg-white/95 backdrop-blur lg:sticky lg:top-0 lg:-mx-10 dark:border-white/10 dark:bg-zinc-950/90">
        <div className="mx-auto grid max-w-7xl gap-4 px-4 py-4 sm:px-6 lg:grid-cols-2 lg:items-center lg:px-8">
          <FilterGroup
            label={t("Time range")}
            options={combatLabPreviewRangeKeys}
            labels={rangeLabels}
            selected={rangeKey}
            onSelect={setRangeKey}
          />
          <FilterGroup
            label={t("Scenario")}
            options={combatLabPreviewScenarioKeys}
            labels={scenarioLabels}
            selected={scenarioKey}
            onSelect={setScenarioKey}
          />
        </div>
      </div>

      <div className="mx-auto max-w-7xl space-y-10 px-4 py-7 sm:px-6 sm:py-10 lg:px-8">
        {drastc ? (
          <div className="rounded-md border border-amber-300/60 bg-amber-50 px-4 py-3 text-amber-950 dark:border-amber-300/20 dark:bg-amber-400/10 dark:text-amber-100">
            <div className="flex items-center gap-3">
              <InformationCircleIcon className="size-5 shrink-0 text-amber-600 dark:text-amber-300" />
              <Text className="!text-sm/5 !text-amber-950 dark:!text-amber-100">
                <strong>{t("Changing the filters will not change the DRASTC results.")}</strong>{" "}
                {t("DRASTC uses the most recent year of open-field battle reports.")}
              </Text>
            </div>
          </div>
        ) : null}

        <section aria-labelledby="combat-breakdown-title">
          <Heading id="combat-breakdown-title" level={2} className="mb-4">
            {t("Combat breakdown")}
          </Heading>
          {hasCombatSummary ? (
            <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
              <MetricCard
                available={hasSummaryMetric("battles")}
                label={t("Battle reports")}
                value={formatCount(summary.battles, compactFormatter, numberFormatter)}
                exactValue={numberFormatter.format(summary.battles)}
              />
              <MetricCard
                available={hasSummaryMetric("killPointsGained")}
                label={t("Kill points gained")}
                value={formatCount(summary.killPointsGained, compactFormatter, numberFormatter)}
                exactValue={numberFormatter.format(Math.round(summary.killPointsGained))}
                accent="text-blue-600 dark:text-blue-400"
              />
              <MetricCard
                available={hasSummaryMetric("killPointsLost")}
                label={t("Kill points lost")}
                value={formatCount(summary.killPointsLost, compactFormatter, numberFormatter)}
                exactValue={numberFormatter.format(Math.round(summary.killPointsLost))}
                accent="text-rose-600 dark:text-rose-400"
              />
              <MetricCard
                available={hasSummaryMetric("weightedTradePercent")}
                label={t("Trade percentage")}
                value={`${decimalFormatter.format(summary.weightedTradePercent)}%`}
                accent="text-emerald-700 dark:text-emerald-400"
              />
              <MetricCard
                available={hasSummaryMetric("averageBattleDurationSeconds")}
                label={t("Avg. battle duration")}
                value={formatDuration(summary.averageBattleDurationSeconds, decimalFormatter)}
              />
              <MetricCard
                available={hasSummaryMetric("uniqueGovernors")}
                label={t("Governors")}
                value={formatCount(summary.uniqueGovernors, compactFormatter, numberFormatter)}
                exactValue={numberFormatter.format(summary.uniqueGovernors)}
              />
              <MetricCard
                available={hasSummaryMetric("severelyWoundedInflicted")}
                label={t("Severely wounded inflicted")}
                value={formatCount(
                  summary.severelyWoundedInflicted,
                  compactFormatter,
                  numberFormatter
                )}
                exactValue={numberFormatter.format(Math.round(summary.severelyWoundedInflicted))}
              />
              <MetricCard
                available={hasSummaryMetric("severelyWoundedTaken")}
                label={t("Severely wounded taken")}
                value={formatCount(summary.severelyWoundedTaken, compactFormatter, numberFormatter)}
                exactValue={numberFormatter.format(Math.round(summary.severelyWoundedTaken))}
              />
              <MetricCard
                available={hasSummaryMetric("dps")}
                label={t("Damage per second (DPS)")}
                value={decimalFormatter.format(summary.dps)}
              />
              <MetricCard
                available={hasSummaryMetric("sps")}
                label={t("Sevs per second (SPS)")}
                value={decimalFormatter.format(summary.sps)}
              />
              <MetricCard
                available={hasSummaryMetric("tps")}
                label={t("Sevs taken per second (TPS)")}
                value={decimalFormatter.format(summary.tps)}
              />
              <MetricCard
                available={hasSummaryMetric("hps")}
                label={t("Healing per second (HPS)")}
                value={decimalFormatter.format(summary.hps)}
              />
            </div>
          ) : (
            <CombatLabPreviewEmptyState
              message={t("No combat summary was observed for this time range and scenario.")}
            />
          )}
        </section>

        <section aria-labelledby="trends-title">
          <Heading id="trends-title" level={2} className="mb-4">
            {t("Combat performance")}
          </Heading>
          <CombatLabPreviewCharts rangeKey={rangeKey} trends={trends} />
        </section>

        {drastc ? <CombatLabPreviewDrastc score={drastc} /> : null}

        <CombatLabPreviewLoadouts
          formationUsage={formationUsage}
          loadouts={selected?.loadouts}
          rangeKey={rangeKey}
        />
      </div>
    </div>
  );
}

function FilterGroup<T extends string>({
  label,
  labels,
  onSelect,
  options,
  selected,
}: {
  label: string;
  labels: Record<T, string>;
  onSelect: (value: T) => void;
  options: readonly T[];
  selected: T;
}) {
  return (
    <fieldset className="min-w-0">
      <legend className="mb-1.5 font-semibold text-xs uppercase tracking-wider text-zinc-500">
        {label}
      </legend>
      <div className="flex gap-1 overflow-x-auto rounded-lg bg-zinc-950/5 p-1 dark:bg-white/5">
        {options.map((option) => (
          <button
            key={option}
            aria-pressed={selected === option}
            className="shrink-0 rounded-md px-3 py-1.5 font-medium text-sm text-zinc-600 transition hover:text-zinc-950 aria-pressed:bg-white aria-pressed:text-zinc-950 dark:text-zinc-300 dark:hover:text-white dark:aria-pressed:bg-zinc-700 dark:aria-pressed:text-white"
            onClick={() => onSelect(option)}
            type="button"
          >
            {labels[option]}
          </button>
        ))}
      </div>
    </fieldset>
  );
}

function MetricCard({
  accent = "text-zinc-950 dark:text-white",
  available = true,
  exactValue,
  label,
  value,
}: {
  accent?: string;
  available?: boolean;
  exactValue?: string;
  label: string;
  value: string;
}) {
  const t = useExtracted();

  return (
    <article className="rounded-md border border-zinc-950/10 bg-white/70 px-4 py-3.5 dark:border-white/10 dark:bg-white/[.035]">
      <Text className="!text-sm/5">{label}</Text>
      <div
        className={`mt-1.5 font-semibold text-2xl tracking-tight tabular-nums ${available ? accent : "text-zinc-400 dark:text-zinc-500"}`}
        title={available ? exactValue : undefined}
      >
        {available ? value : t("No data")}
      </div>
    </article>
  );
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function normalizeSummary(summary: CombatLabPreviewSummary | undefined): CombatLabPreviewSummary {
  return {
    battles: isFiniteNumber(summary?.battles) ? summary.battles : 0,
    uniqueGovernors: isFiniteNumber(summary?.uniqueGovernors) ? summary.uniqueGovernors : 0,
    killPointsGained: isFiniteNumber(summary?.killPointsGained) ? summary.killPointsGained : 0,
    killPointsLost: isFiniteNumber(summary?.killPointsLost) ? summary.killPointsLost : 0,
    severelyWoundedInflicted: isFiniteNumber(summary?.severelyWoundedInflicted)
      ? summary.severelyWoundedInflicted
      : 0,
    severelyWoundedTaken: isFiniteNumber(summary?.severelyWoundedTaken)
      ? summary.severelyWoundedTaken
      : 0,
    averageBattleDurationSeconds: isFiniteNumber(summary?.averageBattleDurationSeconds)
      ? summary.averageBattleDurationSeconds
      : 0,
    weightedTradePercent: isFiniteNumber(summary?.weightedTradePercent)
      ? summary.weightedTradePercent
      : 0,
    dps: isFiniteNumber(summary?.dps) ? summary.dps : 0,
    sps: isFiniteNumber(summary?.sps) ? summary.sps : 0,
    tps: isFiniteNumber(summary?.tps) ? summary.tps : 0,
    hps: isFiniteNumber(summary?.hps) ? summary.hps : 0,
  };
}

function isCompleteDrastc(score: DrastcData | null | undefined): score is DrastcData {
  return Boolean(
    score &&
      isFiniteNumber(score.overall) &&
      isFiniteNumber(score.samples) &&
      isFiniteNumber(score.confidence?.score) &&
      isFiniteNumber(score.confidence?.unique_governors) &&
      drastcCategoryKeys.every((key) => isFiniteNumber(score.breakdown?.[key]?.score))
  );
}

function formatCount(
  value: number,
  compactFormatter: Intl.NumberFormat,
  numberFormatter: Intl.NumberFormat
) {
  return Math.abs(value) >= 1_000
    ? compactFormatter.format(value)
    : numberFormatter.format(Math.round(value));
}

function formatDuration(totalSeconds: number, formatter: Intl.NumberFormat) {
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return minutes > 0
    ? `${formatter.format(minutes)}m ${formatter.format(seconds)}s`
    : `${formatter.format(seconds)}s`;
}

function ChartSkeleton() {
  return (
    <div className="grid animate-pulse gap-5 xl:grid-cols-2">
      {["kill-points", "battle-tempo"].map((chart) => (
        <div
          className="overflow-hidden rounded-md border border-zinc-950/10 bg-white dark:border-white/10 dark:bg-zinc-900"
          key={chart}
        >
          <div className="border-zinc-950/10 border-b px-5 py-4 dark:border-white/10">
            <SkeletonBlock className="h-5 w-28" />
            <SkeletonBlock className="mt-4 h-9 w-full max-w-sm" />
          </div>
          <div className="flex h-72 items-end gap-4 px-6 pt-8 pb-6">
            {skeletonChartBarHeights.map((height) => (
              <SkeletonBlock
                className="flex-1"
                key={`${chart}-bar-${height}`}
                style={{ height: `${height}%` }}
              />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

function SkeletonBlock({ className = "", style }: { className?: string; style?: CSSProperties }) {
  return <div className={`rounded bg-zinc-200 dark:bg-zinc-800 ${className}`} style={style} />;
}
