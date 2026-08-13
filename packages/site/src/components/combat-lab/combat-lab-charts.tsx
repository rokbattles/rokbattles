"use client";

import { useExtracted, useLocale } from "next-intl";
import { useMemo, useState } from "react";
import {
  Area,
  AreaChart,
  CartesianGrid,
  Line,
  LineChart,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  type TooltipContentProps,
  XAxis,
  YAxis,
} from "recharts";
import { CombatLabDonut, type CombatLabDonutDatum } from "@/components/combat-lab/combat-lab-donut";
import { CombatLabEmptyState } from "@/components/combat-lab/combat-lab-empty-state";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { getArmamentInfo } from "@/hooks/use-armament-name";
import { getEquipmentName } from "@/hooks/use-equipment-name";
import { getFormationName } from "@/hooks/use-formation-name";
import type {
  CombatLabPreviewAccessoryPairings,
  CombatLabPreviewArmamentSlot,
  CombatLabPreviewEquipmentSlot,
  CombatLabPreviewFormationUsagePoint,
  CombatLabPreviewRangeKey,
  CombatLabPreviewTrend,
} from "@/lib/combat-lab/preview-types";
import { calculateTradePercentage } from "@/lib/combat-lab/trade-percentage";
import { toRomanNumeral } from "@/lib/equipment";

const DAY_MS = 24 * 60 * 60 * 1_000;
const formationColors = [
  "#0891b2",
  "#2563eb",
  "#7c3aed",
  "#e11d48",
  "#059669",
  "#d97706",
  "#9333ea",
  "#db2777",
  "#65a30d",
  "#ea580c",
  "#4f46e5",
  "#0d9488",
  "#ca8a04",
  "#c026d3",
  "#0284c7",
  "#16a34a",
] as const;
const equipmentColors = ["#2563eb", "#7c3aed", "#0891b2", "#059669", "#d97706"] as const;
const iconicColors = ["#a1a1aa", "#2563eb", "#0891b2", "#7c3aed", "#d97706", "#e11d48"] as const;

type MetricKey = "dps" | "sps" | "tps" | "hps";
type InscriptionKey =
  | "special"
  | "rare"
  | "common"
  | "specialCommon"
  | "rareCommon"
  | "commonCommon";

function formatBattleTempoValue(
  value: number,
  compactFormatter: Intl.NumberFormat,
  decimalFormatter: Intl.NumberFormat
) {
  return Math.abs(value) >= 1_000 ? compactFormatter.format(value) : decimalFormatter.format(value);
}

function formatCount(
  value: number,
  compactFormatter: Intl.NumberFormat,
  integerFormatter: Intl.NumberFormat
) {
  return Math.abs(value) >= 1_000
    ? compactFormatter.format(value)
    : integerFormatter.format(Math.round(value));
}

function numberOrZero(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

export function CombatLabCharts({
  rangeKey,
  trends,
}: {
  rangeKey: CombatLabPreviewRangeKey;
  trends: CombatLabPreviewTrend[];
}) {
  const t = useExtracted();
  const locale = useLocale();
  const compactFormatter = createCompactFormatter(locale);
  const decimalFormatter = createDecimalFormatter(locale);
  const integerFormatter = createIntegerFormatter(locale);
  const metricOptions = [
    { key: "dps", label: t("Damage dealt"), color: "#2563eb" },
    { key: "sps", label: t("Severely wounded inflicted"), color: "#7c3aed" },
    { key: "tps", label: t("Severely wounded taken"), color: "#e11d48" },
    { key: "hps", label: t("Healing"), color: "#059669" },
  ] as const;
  const [metric, setMetric] = useState<MetricKey>("dps");
  const chartData = useMemo(
    () => aggregateTrends(trends, rangeKey, locale),
    [locale, rangeKey, trends]
  );
  const selectedMetric = metricOptions.find((option) => option.key === metric) ?? metricOptions[0];

  if (!chartData.some((point) => point.battles > 0)) {
    return (
      <div className="grid gap-5 xl:grid-cols-2">
        <EmptyChartCard title={t("Kill points")} />
        <EmptyChartCard title={t("Battle tempo")} />
        <EmptyChartCard className="xl:col-span-2" title={t("Trade percentage")} />
      </div>
    );
  }

  return (
    <div className="grid gap-5 xl:grid-cols-2">
      <article className="min-w-0 overflow-hidden rounded-md border border-zinc-950/10 bg-white dark:border-white/10 dark:bg-zinc-900">
        <div className="border-zinc-950/10 border-b px-5 py-4 dark:border-white/10">
          <Subheading level={3}>{t("Kill points")}</Subheading>
          <div className="mt-4 flex h-9 items-center gap-3 text-xs font-medium">
            <ChartLegend color="bg-blue-600" label={t("Gained")} />
            <ChartLegend color="bg-rose-500" label={t("Lost")} />
          </div>
        </div>
        <div className="h-72 px-2 pt-5 pr-4 pb-3">
          <ResponsiveContainer minWidth={0}>
            <AreaChart data={chartData} margin={{ top: 4, right: 4, bottom: 4, left: 4 }}>
              <defs>
                <linearGradient id="kp-gained" x1="0" x2="0" y1="0" y2="1">
                  <stop offset="0%" stopColor="#2563eb" stopOpacity={0.3} />
                  <stop offset="100%" stopColor="#2563eb" stopOpacity={0.02} />
                </linearGradient>
                <linearGradient id="kp-lost" x1="0" x2="0" y1="0" y2="1">
                  <stop offset="0%" stopColor="#f43f5e" stopOpacity={0.2} />
                  <stop offset="100%" stopColor="#f43f5e" stopOpacity={0.01} />
                </linearGradient>
              </defs>
              <CartesianGrid
                stroke="rgba(113,113,122,.18)"
                strokeDasharray="3 5"
                vertical={false}
              />
              <XAxis
                axisLine={false}
                dataKey="date"
                minTickGap={42}
                tick={{ fill: "#71717a", fontSize: 11 }}
                tickLine={false}
              />
              <YAxis
                axisLine={false}
                tick={{ fill: "#71717a", fontSize: 11 }}
                tickFormatter={(value) =>
                  formatCount(Number(value), compactFormatter, integerFormatter)
                }
                tickLine={false}
                width={46}
              />
              <RechartsTooltip
                content={(props) => (
                  <ChartTooltip
                    {...props}
                    formatName={(dataKey) =>
                      dataKey === "killPointsGained" ? t("Gained") : t("Lost")
                    }
                    formatValue={(value) => formatCount(value, compactFormatter, integerFormatter)}
                  />
                )}
              />
              <Area
                dataKey="killPointsGained"
                dot={false}
                fill="url(#kp-gained)"
                isAnimationActive={false}
                stroke="#2563eb"
                strokeWidth={2.25}
                type="monotone"
              />
              <Area
                dataKey="killPointsLost"
                dot={false}
                fill="url(#kp-lost)"
                isAnimationActive={false}
                stroke="#f43f5e"
                strokeWidth={2}
                type="monotone"
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </article>

      <article className="min-w-0 overflow-hidden rounded-md border border-zinc-950/10 bg-white dark:border-white/10 dark:bg-zinc-900">
        <div className="border-zinc-950/10 border-b px-5 py-4 dark:border-white/10">
          <Subheading level={3}>{t("Battle tempo")}</Subheading>
          <div className="mt-4 flex h-9 gap-1 overflow-x-auto rounded-lg bg-zinc-950/5 p-1 dark:bg-white/5">
            {metricOptions.map((option) => (
              <button
                key={option.key}
                aria-pressed={metric === option.key}
                className="shrink-0 rounded-md px-2.5 py-1.5 font-medium text-xs text-zinc-600 transition data-[active=true]:bg-white data-[active=true]:text-zinc-950 dark:text-zinc-300 dark:data-[active=true]:bg-zinc-700 dark:data-[active=true]:text-white"
                data-active={metric === option.key}
                onClick={() => setMetric(option.key)}
                type="button"
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>
        <div className="h-72 px-2 pt-5 pr-4 pb-3">
          <ResponsiveContainer minWidth={0}>
            <LineChart data={chartData} margin={{ top: 4, right: 4, bottom: 4, left: 4 }}>
              <CartesianGrid
                stroke="rgba(113,113,122,.18)"
                strokeDasharray="3 5"
                vertical={false}
              />
              <XAxis
                axisLine={false}
                dataKey="date"
                minTickGap={42}
                tick={{ fill: "#71717a", fontSize: 11 }}
                tickLine={false}
              />
              <YAxis
                axisLine={false}
                tick={{ fill: "#71717a", fontSize: 11 }}
                tickFormatter={(value) =>
                  formatBattleTempoValue(Number(value), compactFormatter, decimalFormatter)
                }
                tickLine={false}
                width={46}
              />
              <RechartsTooltip
                content={(props) => (
                  <ChartTooltip
                    {...props}
                    formatName={() => selectedMetric.label}
                    formatValue={(value) =>
                      formatBattleTempoValue(value, compactFormatter, decimalFormatter)
                    }
                  />
                )}
              />
              <Line
                dataKey={metric}
                dot={false}
                isAnimationActive={false}
                stroke={selectedMetric.color}
                strokeWidth={2.5}
                type="monotone"
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </article>

      <article
        aria-labelledby="trade-percentage-chart-title"
        className="min-w-0 overflow-hidden rounded-md border border-zinc-950/10 bg-white xl:col-span-2 dark:border-white/10 dark:bg-zinc-900"
        data-testid="trade-percentage-chart"
      >
        <div className="border-zinc-950/10 border-b px-5 py-4 dark:border-white/10">
          <Subheading id="trade-percentage-chart-title" level={3}>
            {t("Trade percentage")}
          </Subheading>
        </div>
        <div className="h-72 px-2 pt-5 pr-4 pb-3">
          <ResponsiveContainer minWidth={0}>
            <AreaChart
              accessibilityLayer
              data={chartData}
              margin={{ top: 4, right: 4, bottom: 4, left: 4 }}
            >
              <defs>
                <linearGradient id="trade-percentage" x1="0" x2="0" y1="0" y2="1">
                  <stop offset="0%" stopColor="#059669" stopOpacity={0.3} />
                  <stop offset="100%" stopColor="#059669" stopOpacity={0.02} />
                </linearGradient>
              </defs>
              <CartesianGrid
                stroke="rgba(113,113,122,.18)"
                strokeDasharray="3 5"
                vertical={false}
              />
              <XAxis
                axisLine={false}
                dataKey="date"
                minTickGap={42}
                tick={{ fill: "#71717a", fontSize: 11 }}
                tickLine={false}
              />
              <YAxis
                axisLine={false}
                domain={[0, "auto"]}
                tick={{ fill: "#71717a", fontSize: 11 }}
                tickFormatter={(value) => `${decimalFormatter.format(Number(value))}%`}
                tickLine={false}
                width={58}
              />
              <RechartsTooltip
                content={(props) => (
                  <ChartTooltip
                    {...props}
                    formatName={() => t("Trade percentage")}
                    formatValue={(value) => `${decimalFormatter.format(value)}%`}
                  />
                )}
              />
              <Area
                dataKey="tradePercentage"
                dot={false}
                fill="url(#trade-percentage)"
                isAnimationActive={false}
                stroke="#059669"
                strokeWidth={2.5}
                type="monotone"
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </article>
    </div>
  );
}

function EmptyChartCard({ className, title }: { className?: string; title: string }) {
  return (
    <article
      className={`min-w-0 overflow-hidden rounded-md border border-zinc-950/10 bg-white dark:border-white/10 dark:bg-zinc-900 ${className ?? ""}`}
    >
      <div className="border-zinc-950/10 border-b px-5 py-4 dark:border-white/10">
        <Subheading level={3}>{title}</Subheading>
      </div>
      <CombatLabEmptyState className="m-5 min-h-64" />
    </article>
  );
}

export function CombatLabFormationChart({
  points,
  rangeKey,
}: {
  points: CombatLabPreviewFormationUsagePoint[];
  rangeKey: CombatLabPreviewRangeKey;
}) {
  const t = useExtracted();
  const locale = useLocale();
  const decimalFormatter = createDecimalFormatter(locale);
  const { chartData, series } = useMemo(
    () =>
      aggregateFormationUsage(points, rangeKey, locale, (id) =>
        t("Formation {id}", { id: id.toString() })
      ),
    [locale, points, rangeKey, t]
  );
  const seriesByKey = useMemo(
    () => new Map(series.map((formation) => [formation.dataKey, formation.name])),
    [series]
  );

  if (chartData.length === 0 || series.length === 0) {
    return (
      <article className="min-w-0 rounded-md border border-zinc-950/10 bg-white p-5 xl:col-span-2 dark:border-white/10 dark:bg-zinc-900 sm:p-6">
        <Subheading level={3} className="!text-lg/7">
          {t("Formation")}
        </Subheading>
        <Subheading level={4} className="mt-3">
          {t("Formation usage")}
        </Subheading>
        <CombatLabEmptyState className="mt-3 min-h-72 sm:min-h-80" />
      </article>
    );
  }

  return (
    <article className="min-w-0 rounded-md border border-zinc-950/10 bg-white p-5 xl:col-span-2 dark:border-white/10 dark:bg-zinc-900 sm:p-6">
      <Subheading level={3} className="!text-lg/7">
        {t("Formation")}
      </Subheading>
      <div className="mt-3">
        <Subheading level={4}>{t("Formation usage")}</Subheading>
        <div className="mt-3 flex flex-wrap gap-x-4 gap-y-2 text-xs font-medium">
          {series.map((formation) => (
            <span
              key={formation.id}
              className="inline-flex items-center gap-1.5 text-zinc-600 dark:text-zinc-300"
            >
              <span className="size-2 rounded-full" style={{ backgroundColor: formation.color }} />
              {formation.name}
            </span>
          ))}
        </div>
        <div className="mt-2 h-80 sm:h-96">
          <ResponsiveContainer minWidth={0}>
            <LineChart data={chartData} margin={{ top: 4, right: 4, bottom: 4, left: 4 }}>
              <CartesianGrid
                stroke="rgba(113,113,122,.18)"
                strokeDasharray="3 5"
                vertical={false}
              />
              <XAxis
                axisLine={false}
                dataKey="date"
                minTickGap={42}
                tick={{ fill: "#71717a", fontSize: 11 }}
                tickLine={false}
              />
              <YAxis
                axisLine={false}
                domain={[0, 100]}
                tick={{ fill: "#71717a", fontSize: 11 }}
                tickFormatter={(value) => `${Number(value).toFixed(0)}%`}
                tickLine={false}
                width={46}
              />
              <RechartsTooltip
                content={(props) => (
                  <ChartTooltip
                    {...props}
                    formatName={(dataKey) =>
                      seriesByKey.get(String(dataKey)) ?? t("Unknown formation")
                    }
                    formatValue={(value) => `${decimalFormatter.format(value)}%`}
                  />
                )}
              />
              {series.map((formation) => (
                <Line
                  key={formation.id}
                  dataKey={formation.dataKey}
                  dot={false}
                  isAnimationActive={false}
                  name={formation.name}
                  stroke={formation.color}
                  strokeWidth={formation.id === 2 ? 2.75 : 1.75}
                  type="monotone"
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>
    </article>
  );
}

export function CombatLabArmamentChart({
  rangeKey,
  slot,
}: {
  rangeKey: CombatLabPreviewRangeKey;
  slot: CombatLabPreviewArmamentSlot;
}) {
  const t = useExtracted();
  const locale = useLocale();
  const decimalFormatter = createDecimalFormatter(locale);
  const inscriptionOptions = useMemo(
    () =>
      [
        { key: "special", label: t("Special"), color: "#2563eb" },
        { key: "rare", label: t("Rare"), color: "#7c3aed" },
        { key: "common", label: t("Common"), color: "#71717a" },
        { key: "specialCommon", label: t("Special + Common"), color: "#0891b2" },
        { key: "rareCommon", label: t("Rare + Common"), color: "#d97706" },
        { key: "commonCommon", label: t("Common + Common"), color: "#059669" },
      ] as const,
    [t]
  );
  const buffOptions = useMemo(
    () => getArmamentBuffOptions(slot, locale, (id) => t("Buff {id}", { id: id.toString() })),
    [locale, slot, t]
  );
  const [requestedBuffId, setRequestedBuffId] = useState<number | null>(null);
  const selectedBuffId = buffOptions.some((buff) => buff.id === requestedBuffId)
    ? requestedBuffId
    : (buffOptions[0]?.id ?? null);
  const { chartData, inscriptionSeries } = useMemo(
    () => aggregateArmamentUsage(slot, rangeKey, selectedBuffId, locale, inscriptionOptions),
    [inscriptionOptions, locale, rangeKey, selectedBuffId, slot]
  );
  const selectedBuff = buffOptions.find((buff) => buff.id === selectedBuffId);
  const selectedBuffInfo = selectedBuff ? getArmamentInfo(selectedBuff.id, locale) : undefined;

  if (!slot.points.some((point) => point.sampleSize > 0)) {
    return <CombatLabEmptyState className="min-h-[34rem]" />;
  }

  return (
    <div className="space-y-7">
      <div>
        <Subheading level={4}>{t("Inscription usage")}</Subheading>
        <div className="mt-3 flex min-h-9 flex-wrap gap-x-3 gap-y-2 text-xs font-medium">
          {inscriptionSeries.map((series) => (
            <span
              className="inline-flex items-center gap-1.5 text-zinc-600 dark:text-zinc-300"
              key={series.key}
            >
              <span className="size-2 rounded-full" style={{ backgroundColor: series.color }} />
              {series.label}
            </span>
          ))}
        </div>
        <div className="mt-2 h-52">
          <ResponsiveContainer minWidth={0}>
            <LineChart data={chartData} margin={{ top: 4, right: 4, bottom: 4, left: 0 }}>
              <CartesianGrid
                stroke="rgba(113,113,122,.18)"
                strokeDasharray="3 5"
                vertical={false}
              />
              <XAxis
                axisLine={false}
                dataKey="date"
                minTickGap={36}
                tick={{ fill: "#71717a", fontSize: 10 }}
                tickLine={false}
              />
              <YAxis
                axisLine={false}
                domain={[0, 100]}
                tick={{ fill: "#71717a", fontSize: 10 }}
                tickFormatter={(value) => `${Number(value).toFixed(0)}%`}
                tickLine={false}
                width={40}
              />
              <RechartsTooltip
                content={(props) => (
                  <ChartTooltip
                    {...props}
                    formatName={(dataKey) =>
                      inscriptionOptions.find((option) => option.key === dataKey)?.label ??
                      t("Inscription")
                    }
                    formatValue={(value) => `${decimalFormatter.format(value)}%`}
                  />
                )}
              />
              {inscriptionSeries.map((series) => (
                <Line
                  dataKey={series.key}
                  dot={false}
                  isAnimationActive={false}
                  key={series.key}
                  stroke={series.color}
                  strokeWidth={series.key === "common" ? 2.5 : 1.75}
                  type="monotone"
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>

      <div>
        <Subheading level={4}>{t("Buff rolls")}</Subheading>
        <div className="mt-3 flex gap-1 overflow-x-auto rounded-lg bg-zinc-950/5 p-1 dark:bg-white/5">
          {buffOptions.map((buff) => (
            <button
              aria-pressed={selectedBuffId === buff.id}
              className="shrink-0 rounded-md px-2.5 py-1.5 font-medium text-xs text-zinc-600 transition data-[active=true]:bg-white data-[active=true]:text-zinc-950 dark:text-zinc-300 dark:data-[active=true]:bg-zinc-700 dark:data-[active=true]:text-white"
              data-active={selectedBuffId === buff.id}
              key={buff.id}
              onClick={() => setRequestedBuffId(buff.id)}
              type="button"
            >
              {buff.name}
            </button>
          ))}
        </div>
        <div className="mt-2 h-52">
          {selectedBuffId === null ? (
            <Text className="flex h-full items-center justify-center !text-sm">
              {t("No buff rolls observed.")}
            </Text>
          ) : (
            <ResponsiveContainer minWidth={0}>
              <LineChart data={chartData} margin={{ top: 4, right: 4, bottom: 4, left: 0 }}>
                <CartesianGrid
                  stroke="rgba(113,113,122,.18)"
                  strokeDasharray="3 5"
                  vertical={false}
                />
                <XAxis
                  axisLine={false}
                  dataKey="date"
                  minTickGap={36}
                  tick={{ fill: "#71717a", fontSize: 10 }}
                  tickLine={false}
                />
                <YAxis
                  allowDataOverflow
                  axisLine={false}
                  domain={[0, selectedBuff?.maximumRoll ?? "auto"]}
                  tick={{ fill: "#71717a", fontSize: 10 }}
                  tickFormatter={(value) =>
                    formatArmamentValue(Number(value), decimalFormatter, selectedBuffInfo?.percent)
                  }
                  tickLine={false}
                  width={48}
                />
                <RechartsTooltip
                  content={(props) => (
                    <ArmamentBuffTooltip
                      {...props}
                      averageRollLabel={t("Average roll")}
                      buffName={selectedBuff?.name ?? t("Buff")}
                      decimalFormatter={decimalFormatter}
                      maxRollUsageLabel={t("Max roll usage")}
                      percent={selectedBuffInfo?.percent}
                      usageLabel={t("Usage")}
                    />
                  )}
                />
                <Line
                  dataKey="buffAverage"
                  dot={false}
                  isAnimationActive={false}
                  stroke="#7c3aed"
                  strokeWidth={2.5}
                  type="monotone"
                />
              </LineChart>
            </ResponsiveContainer>
          )}
        </div>
      </div>
    </div>
  );
}

export function CombatLabEquipmentChart({
  accessoryPairings,
  rangeKey,
  slot,
}: {
  accessoryPairings?: CombatLabPreviewAccessoryPairings;
  rangeKey: CombatLabPreviewRangeKey;
  slot: CombatLabPreviewEquipmentSlot;
}) {
  const t = useExtracted();
  const locale = useLocale();
  const compactFormatter = createCompactFormatter(locale);
  const decimalFormatter = createDecimalFormatter(locale);
  const integerFormatter = createIntegerFormatter(locale);
  const { chartData, iconicData, nonLegendaryCount, series, specialTalentData, totals } = useMemo(
    () =>
      aggregateEquipmentUsage(slot, rangeKey, locale, {
        iconic: (level) => `${t("Iconic")} ${level}`,
        item: (id) => t("Item {id}", { id: id.toString() }),
        noIconic: t("No iconic"),
        noSpecialTalent: t("No special talent"),
        otherPieces: t("Other pieces"),
        specialTalent: t("Special talent"),
      }),
    [locale, rangeKey, slot, t]
  );
  const namesByKey = useMemo(
    () => new Map(series.map((item) => [item.dataKey, item.name])),
    [series]
  );
  const specialTalentPercent =
    totals.observations > 0 ? (totals.specialTalent / totals.observations) * 100 : 0;
  const accessoryPairingData = useMemo(
    () =>
      getAccessoryPairingData(accessoryPairings, locale, {
        item: (id) => t("Item {id}", { id: id.toString() }),
        otherPairings: t("Other pairings"),
      }),
    [accessoryPairings, locale, t]
  );

  if (totals.observations <= 0) {
    return <CombatLabEmptyState className="min-h-[31rem]" />;
  }

  return (
    <div className="space-y-7">
      <div>
        <Subheading level={4}>{t("Piece usage")}</Subheading>
        <div className="mt-3 flex min-h-9 flex-wrap gap-x-3 gap-y-2 text-xs font-medium">
          {series.map((item) => (
            <span
              className="inline-flex min-w-0 items-center gap-1.5 text-zinc-600 dark:text-zinc-300"
              key={item.dataKey}
              title={item.name}
            >
              <span
                className="size-2 shrink-0 rounded-full"
                style={{ backgroundColor: item.color }}
              />
              <span className="max-w-44 truncate">{item.name}</span>
            </span>
          ))}
        </div>
        <div className="mt-2 h-52">
          <ResponsiveContainer minWidth={0}>
            <LineChart data={chartData} margin={{ top: 4, right: 4, bottom: 4, left: 0 }}>
              <CartesianGrid
                stroke="rgba(113,113,122,.18)"
                strokeDasharray="3 5"
                vertical={false}
              />
              <XAxis
                axisLine={false}
                dataKey="date"
                minTickGap={36}
                tick={{ fill: "#71717a", fontSize: 10 }}
                tickLine={false}
              />
              <YAxis
                axisLine={false}
                domain={[0, 100]}
                tick={{ fill: "#71717a", fontSize: 10 }}
                tickFormatter={(value) => `${Number(value).toFixed(0)}%`}
                tickLine={false}
                width={40}
              />
              <RechartsTooltip
                content={(props) => (
                  <ChartTooltip
                    {...props}
                    formatName={(dataKey) => namesByKey.get(String(dataKey)) ?? t("Equipment")}
                    formatValue={(value) => `${decimalFormatter.format(value)}%`}
                  />
                )}
              />
              {series.map((item) => (
                <Line
                  dataKey={item.dataKey}
                  dot={false}
                  isAnimationActive={false}
                  key={item.dataKey}
                  stroke={item.color}
                  strokeWidth={item.isOther ? 1.5 : 2}
                  strokeDasharray={item.isOther ? "4 4" : undefined}
                  type="monotone"
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>

      <div className={`grid gap-6 ${slot.slot === 7 ? "sm:grid-cols-3" : "sm:grid-cols-2"}`}>
        <CombatLabDonut
          ariaLabel={t("Iconic level distribution among legendary equipment")}
          centerLabel={t("legendary")}
          centerValue={formatCount(totals.legendary, compactFormatter, integerFormatter)}
          data={iconicData}
          emptyLabel={t("No legendary pieces observed.")}
          integerFormatter={integerFormatter}
          title={t("Iconic levels")}
          tooltipLabels={{ count: t("Count"), usage: t("Usage") }}
          decimalFormatter={decimalFormatter}
        />
        <CombatLabDonut
          ariaLabel={t("Special talent usage across equipment")}
          centerLabel={t("special talent")}
          centerValue={`${decimalFormatter.format(specialTalentPercent)}%`}
          data={specialTalentData}
          emptyLabel={t("No talent data observed.")}
          integerFormatter={integerFormatter}
          title={t("Special talent")}
          tooltipLabels={{ count: t("Count"), usage: t("Usage") }}
          decimalFormatter={decimalFormatter}
        />
        {slot.slot === 7 ? (
          <CombatLabDonut
            ariaLabel={t("Most common order-agnostic accessory pairings")}
            centerLabel={t("pairings")}
            centerValue={formatCount(
              accessoryPairings?.sampleSize ?? 0,
              compactFormatter,
              integerFormatter
            )}
            data={accessoryPairingData}
            emptyLabel={t("No complete accessory pairings observed.")}
            integerFormatter={integerFormatter}
            title={t("Accessory pairings")}
            tooltipLabels={{ count: t("Count"), usage: t("Usage") }}
            decimalFormatter={decimalFormatter}
          />
        ) : null}
      </div>
      <Text className="text-center !text-xs/5">
        {t(
          "{count, plural, one {# piece of equipment was excluded from iconic levels because it was not legendary.} other {# pieces of equipment were excluded from iconic levels because they were not legendary.}}",
          { count: nonLegendaryCount }
        )}
      </Text>
    </div>
  );
}

function ArmamentBuffTooltip({
  active,
  averageRollLabel,
  buffName,
  decimalFormatter,
  label,
  maxRollUsageLabel,
  payload,
  percent,
  usageLabel,
}: TooltipContentProps & {
  averageRollLabel: string;
  buffName: string;
  decimalFormatter: Intl.NumberFormat;
  maxRollUsageLabel: string;
  percent?: boolean;
  usageLabel: string;
}) {
  const row = payload?.[0]?.payload as ArmamentChartData | undefined;
  if (!(active && row && row.buffObservations > 0)) {
    return null;
  }

  return (
    <div
      className="min-w-52 rounded-md border border-zinc-950/10 bg-white px-3 py-2 text-xs text-zinc-950 dark:border-white/10 dark:bg-zinc-900 dark:text-white"
      data-chart-tooltip=""
    >
      <div className="text-zinc-500 dark:text-zinc-400">{label}</div>
      <div className="mt-1.5 font-medium">{buffName}</div>
      <dl className="mt-2 grid grid-cols-[auto_auto] gap-x-5 gap-y-1">
        <dt className="text-zinc-500 dark:text-zinc-400">{usageLabel}</dt>
        <dd className="text-right tabular-nums">{decimalFormatter.format(row.buffUsage)}%</dd>
        <dt className="text-zinc-500 dark:text-zinc-400">{maxRollUsageLabel}</dt>
        <dd className="text-right tabular-nums">{decimalFormatter.format(row.buffMaxPercent)}%</dd>
        <dt className="text-zinc-500 dark:text-zinc-400">{averageRollLabel}</dt>
        <dd className="text-right tabular-nums">
          {formatArmamentValue(row.buffAverage, decimalFormatter, percent)}
        </dd>
      </dl>
    </div>
  );
}

function ChartLegend({ color, label }: { color: string; label: string }) {
  return (
    <span className="inline-flex items-center gap-1.5 text-zinc-600 dark:text-zinc-300">
      <span className={`size-2 rounded-full ${color}`} />
      {label}
    </span>
  );
}

function ChartTooltip({
  active,
  formatName,
  formatValue,
  label,
  payload,
}: TooltipContentProps & {
  formatName: (dataKey: unknown) => string;
  formatValue: (value: number) => string;
}) {
  if (!(active && payload.length > 0)) {
    return null;
  }

  return (
    <div
      className="min-w-32 rounded-md border border-zinc-950/10 bg-white px-3 py-2 text-xs text-zinc-950 dark:border-white/10 dark:bg-zinc-900 dark:text-white"
      data-chart-tooltip=""
    >
      <div className="mb-1.5 text-zinc-500 dark:text-zinc-400">{label}</div>
      <div className="space-y-1">
        {payload.map((entry) => (
          <div
            key={`${entry.graphicalItemId}-${String(entry.dataKey)}`}
            className="flex items-center justify-between gap-4"
          >
            <span className="inline-flex items-center gap-1.5">
              <span
                className="size-2 rounded-full"
                style={{ backgroundColor: entry.color ?? entry.stroke }}
              />
              {formatName(entry.dataKey)}
            </span>
            <span className="font-medium tabular-nums">{formatValue(Number(entry.value))}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

type AggregatedTrend = CombatLabPreviewTrend & { date: string; tradePercentage: number };

function aggregateTrends(
  trends: CombatLabPreviewTrend[],
  rangeKey: CombatLabPreviewRangeKey,
  locale: string
): AggregatedTrend[] {
  const bucketMs = trendBucketDays(rangeKey) * DAY_MS;
  const grouped = new Map<number, CombatLabPreviewTrend>();

  for (const point of trends) {
    const bucket = Math.floor(point.bucketStartMs / bucketMs) * bucketMs;
    const current = grouped.get(bucket);
    if (!current) {
      grouped.set(bucket, { ...point, bucketStartMs: bucket });
      continue;
    }
    const totalBattles = current.battles + point.battles;
    const weighted = (key: "dps" | "sps" | "tps" | "hps") =>
      totalBattles > 0
        ? (current[key] * current.battles + point[key] * point.battles) / totalBattles
        : 0;
    grouped.set(bucket, {
      bucketStartMs: bucket,
      battles: totalBattles,
      killPointsGained: current.killPointsGained + point.killPointsGained,
      killPointsLost: current.killPointsLost + point.killPointsLost,
      dps: weighted("dps"),
      sps: weighted("sps"),
      tps: weighted("tps"),
      hps: weighted("hps"),
    });
  }

  const dateFormatter = trendDateFormatter(locale);
  return Array.from(grouped.values())
    .sort((a, b) => a.bucketStartMs - b.bucketStartMs)
    .map((point) => ({
      ...point,
      date: dateFormatter.format(point.bucketStartMs),
      tradePercentage: calculateTradePercentage(point.killPointsGained, point.killPointsLost),
    }));
}

type FormationChartSeries = {
  id: number;
  dataKey: string;
  name: string;
  color: string;
};

type FormationChartBucket = {
  bucketStartMs: number;
  sampleSize: number;
  counts: Map<number, number>;
};

type FormationChartData = Record<string, number | string> & {
  bucketStartMs: number;
  date: string;
};

function aggregateFormationUsage(
  points: CombatLabPreviewFormationUsagePoint[],
  rangeKey: CombatLabPreviewRangeKey,
  locale: string,
  fallbackName: (id: number) => string
): { chartData: FormationChartData[]; series: FormationChartSeries[] } {
  const bucketMs = trendBucketDays(rangeKey) * DAY_MS;
  const grouped = new Map<number, FormationChartBucket>();
  const totals = new Map<number, number>();

  for (const point of points) {
    const bucketStartMs = Math.floor(point.bucketStartMs / bucketMs) * bucketMs;
    const bucket = grouped.get(bucketStartMs) ?? {
      bucketStartMs,
      sampleSize: 0,
      counts: new Map<number, number>(),
    };
    bucket.sampleSize += numberOrZero(point.sampleSize);
    for (const formation of Array.isArray(point.formations) ? point.formations : []) {
      const count = numberOrZero(formation.count);
      bucket.counts.set(formation.id, (bucket.counts.get(formation.id) ?? 0) + count);
      totals.set(formation.id, (totals.get(formation.id) ?? 0) + count);
    }
    grouped.set(bucketStartMs, bucket);
  }

  const series = Array.from(totals)
    .sort((left, right) => right[1] - left[1] || left[0] - right[0])
    .map(([id]) => ({
      id,
      dataKey: `formation-${id}`,
      name: getFormationName(id, locale) ?? fallbackName(id),
      color: formationColors[(id - 1) % formationColors.length],
    }));
  const dateFormatter = trendDateFormatter(locale);
  const chartData = Array.from(grouped.values())
    .sort((left, right) => left.bucketStartMs - right.bucketStartMs)
    .map((bucket) => {
      const row: FormationChartData = {
        bucketStartMs: bucket.bucketStartMs,
        date: dateFormatter.format(bucket.bucketStartMs),
      };
      for (const formation of series) {
        row[formation.dataKey] =
          bucket.sampleSize > 0
            ? ((bucket.counts.get(formation.id) ?? 0) / bucket.sampleSize) * 100
            : 0;
      }
      return row;
    });

  return { chartData, series };
}

type ArmamentBuffAggregate = {
  observations: number;
  totalRoll: number;
  maximumRoll: number;
  maxRollCount: number;
};

type ArmamentChartBucket = {
  bucketStartMs: number;
  sampleSize: number;
  inscriptions: Record<InscriptionKey, number>;
  buffs: Map<number, ArmamentBuffAggregate>;
};

type ArmamentChartData = Record<InscriptionKey, number> & {
  bucketStartMs: number;
  date: string;
  sampleSize: number;
  buffAverage: number;
  buffUsage: number;
  buffObservations: number;
  buffMaxPercent: number;
};

function getArmamentBuffOptions(
  slot: CombatLabPreviewArmamentSlot,
  locale: string,
  fallbackName: (id: number) => string
) {
  const totals = new Map<number, { maximumRoll: number; observations: number }>();
  for (const point of slot.points) {
    for (const buff of Array.isArray(point.buffs) ? point.buffs : []) {
      const current = totals.get(buff.id) ?? { maximumRoll: 0, observations: 0 };
      current.maximumRoll = Math.max(current.maximumRoll, buff.maximumRoll);
      current.observations += buff.observations;
      totals.set(buff.id, current);
    }
  }

  return Array.from(totals)
    .sort((left, right) => right[1].observations - left[1].observations || left[0] - right[0])
    .slice(0, 4)
    .map(([id, { maximumRoll }]) => ({
      id,
      maximumRoll,
      name: getArmamentInfo(id, locale)?.name ?? fallbackName(id),
    }));
}

function aggregateArmamentUsage(
  slot: CombatLabPreviewArmamentSlot,
  rangeKey: CombatLabPreviewRangeKey,
  selectedBuffId: number | null,
  locale: string,
  inscriptionOptions: readonly { key: InscriptionKey; label: string; color: string }[]
) {
  const bucketMs = trendBucketDays(rangeKey) * DAY_MS;
  const grouped = new Map<number, ArmamentChartBucket>();
  const inscriptionTotals = new Map<InscriptionKey, number>();

  for (const point of slot.points) {
    const bucketStartMs = Math.floor(point.bucketStartMs / bucketMs) * bucketMs;
    const bucket = grouped.get(bucketStartMs) ?? {
      bucketStartMs,
      sampleSize: 0,
      inscriptions: {
        special: 0,
        rare: 0,
        common: 0,
        specialCommon: 0,
        rareCommon: 0,
        commonCommon: 0,
      },
      buffs: new Map<number, ArmamentBuffAggregate>(),
    };
    bucket.sampleSize += numberOrZero(point.sampleSize);

    for (const option of inscriptionOptions) {
      const count = numberOrZero(point.inscriptions?.[option.key]?.count);
      bucket.inscriptions[option.key] += count;
      inscriptionTotals.set(option.key, (inscriptionTotals.get(option.key) ?? 0) + count);
    }
    for (const buff of Array.isArray(point.buffs) ? point.buffs : []) {
      const aggregate = bucket.buffs.get(buff.id) ?? {
        observations: 0,
        totalRoll: 0,
        maximumRoll: buff.maximumRoll,
        maxRollCount: 0,
      };
      aggregate.observations += buff.observations;
      aggregate.totalRoll += buff.averageRoll * buff.observations;
      aggregate.maximumRoll = Math.max(aggregate.maximumRoll, buff.maximumRoll);
      aggregate.maxRollCount += buff.maxRollCount;
      bucket.buffs.set(buff.id, aggregate);
    }
    grouped.set(bucketStartMs, bucket);
  }

  const inscriptionSeries = inscriptionOptions.filter(
    (option) => (inscriptionTotals.get(option.key) ?? 0) > 0
  );
  const dateFormatter = trendDateFormatter(locale);
  const chartData = Array.from(grouped.values())
    .sort((left, right) => left.bucketStartMs - right.bucketStartMs)
    .map((bucket): ArmamentChartData => {
      const buff = selectedBuffId === null ? undefined : bucket.buffs.get(selectedBuffId);
      const inscriptionPercent = (key: InscriptionKey) =>
        bucket.sampleSize > 0 ? (bucket.inscriptions[key] / bucket.sampleSize) * 100 : 0;
      return {
        bucketStartMs: bucket.bucketStartMs,
        date: dateFormatter.format(bucket.bucketStartMs),
        sampleSize: bucket.sampleSize,
        special: inscriptionPercent("special"),
        rare: inscriptionPercent("rare"),
        common: inscriptionPercent("common"),
        specialCommon: inscriptionPercent("specialCommon"),
        rareCommon: inscriptionPercent("rareCommon"),
        commonCommon: inscriptionPercent("commonCommon"),
        buffAverage: buff && buff.observations > 0 ? buff.totalRoll / buff.observations : 0,
        buffUsage:
          buff && bucket.sampleSize > 0 ? (buff.observations / bucket.sampleSize) * 100 : 0,
        buffObservations: buff?.observations ?? 0,
        buffMaxPercent:
          buff && buff.observations > 0 ? (buff.maxRollCount / buff.observations) * 100 : 0,
      };
    });

  return { chartData, inscriptionSeries };
}

type EquipmentChartSeries = {
  dataKey: string;
  name: string;
  color: string;
  isOther: boolean;
};

type EquipmentChartBucket = {
  bucketStartMs: number;
  sampleSize: number;
  items: Map<number, number>;
};

type EquipmentChartData = Record<string, number | string> & {
  bucketStartMs: number;
  date: string;
};

function getAccessoryPairingData(
  accessoryPairings: CombatLabPreviewAccessoryPairings | undefined,
  locale: string,
  labels: { item: (id: number) => string; otherPairings: string }
): CombatLabDonutDatum[] {
  if (
    !accessoryPairings ||
    accessoryPairings.sampleSize === 0 ||
    !Array.isArray(accessoryPairings.pairings)
  ) {
    return [];
  }

  const topPairings = accessoryPairings.pairings
    .toSorted(
      (left, right) =>
        right.count - left.count ||
        left.firstItemId - right.firstItemId ||
        left.secondItemId - right.secondItemId
    )
    .slice(0, 4);
  const topPairingCount = topPairings.reduce((sum, pairing) => sum + pairing.count, 0);
  const data = topPairings.map(
    (pairing, index): CombatLabDonutDatum => ({
      key: `pair-${pairing.firstItemId}-${pairing.secondItemId}`,
      name: `${getEquipmentName(pairing.firstItemId, locale) ?? labels.item(pairing.firstItemId)} + ${getEquipmentName(pairing.secondItemId, locale) ?? labels.item(pairing.secondItemId)}`,
      count: pairing.count,
      color: equipmentColors[index],
    })
  );
  const otherCount = Math.max(0, accessoryPairings.sampleSize - topPairingCount);
  if (otherCount > 0) {
    data.push({
      key: "other-pairings",
      name: labels.otherPairings,
      count: otherCount,
      color: equipmentColors[4],
    });
  }

  return data;
}

function aggregateEquipmentUsage(
  slot: CombatLabPreviewEquipmentSlot,
  rangeKey: CombatLabPreviewRangeKey,
  locale: string,
  labels: {
    iconic: (level: string) => string;
    item: (id: number) => string;
    noIconic: string;
    noSpecialTalent: string;
    otherPieces: string;
    specialTalent: string;
  }
) {
  const bucketMs = trendBucketDays(rangeKey) * DAY_MS;
  const grouped = new Map<number, EquipmentChartBucket>();
  const itemTotals = new Map<number, number>();
  const iconicTotals = new Map<number, number>();
  let observations = 0;
  let legendary = 0;
  let nonLegendaryCount = 0;
  let specialTalent = 0;
  let noSpecialTalent = 0;

  for (const point of slot.points) {
    const bucketStartMs = Math.floor(point.bucketStartMs / bucketMs) * bucketMs;
    const bucket = grouped.get(bucketStartMs) ?? {
      bucketStartMs,
      sampleSize: 0,
      items: new Map<number, number>(),
    };
    bucket.sampleSize += numberOrZero(point.sampleSize);
    observations += numberOrZero(point.sampleSize);
    legendary += numberOrZero(point.legendaryCount);
    nonLegendaryCount += numberOrZero(point.nonLegendaryCount);
    specialTalent += numberOrZero(point.specialTalentCount);
    noSpecialTalent += numberOrZero(point.noSpecialTalentCount);

    for (const item of Array.isArray(point.items) ? point.items : []) {
      const count = numberOrZero(item.count);
      bucket.items.set(item.id, (bucket.items.get(item.id) ?? 0) + count);
      itemTotals.set(item.id, (itemTotals.get(item.id) ?? 0) + count);
    }
    for (const iconic of Array.isArray(point.iconicLevels) ? point.iconicLevels : []) {
      const count = numberOrZero(iconic.count);
      iconicTotals.set(iconic.level, (iconicTotals.get(iconic.level) ?? 0) + count);
    }
    grouped.set(bucketStartMs, bucket);
  }

  const selectedItemIds = Array.from(itemTotals)
    .sort((left, right) => right[1] - left[1] || left[0] - right[0])
    .slice(0, 4)
    .map(([id]) => id);
  const selectedItemSet = new Set(selectedItemIds);
  const selectedTotal = selectedItemIds.reduce((sum, id) => sum + (itemTotals.get(id) ?? 0), 0);
  const otherTotal = Math.max(0, observations - selectedTotal);
  const series: EquipmentChartSeries[] = selectedItemIds.map((id, index) => ({
    dataKey: `equipment-${id}`,
    name: getEquipmentName(id, locale) ?? labels.item(id),
    color: equipmentColors[index],
    isOther: false,
  }));
  if (otherTotal > 0) {
    series.push({
      dataKey: "equipment-other",
      name: labels.otherPieces,
      color: equipmentColors[4],
      isOther: true,
    });
  }

  const dateFormatter = trendDateFormatter(locale);
  const chartData = Array.from(grouped.values())
    .sort((left, right) => left.bucketStartMs - right.bucketStartMs)
    .map((bucket): EquipmentChartData => {
      const row: EquipmentChartData = {
        bucketStartMs: bucket.bucketStartMs,
        date: dateFormatter.format(bucket.bucketStartMs),
      };
      for (const id of selectedItemIds) {
        const count = bucket.items.get(id) ?? 0;
        row[`equipment-${id}`] = bucket.sampleSize > 0 ? (count / bucket.sampleSize) * 100 : 0;
      }
      if (otherTotal > 0) {
        const otherCount = Array.from(bucket.items).reduce(
          (sum, [id, count]) => sum + (selectedItemSet.has(id) ? 0 : count),
          0
        );
        row["equipment-other"] = bucket.sampleSize > 0 ? (otherCount / bucket.sampleSize) * 100 : 0;
      }
      return row;
    });

  const iconicData = Array.from(iconicTotals)
    .sort((left, right) => left[0] - right[0])
    .map(
      ([level, count]): CombatLabDonutDatum => ({
        key: `iconic-${level}`,
        name:
          level === 0 ? labels.noIconic : labels.iconic(toRomanNumeral(level) ?? level.toString()),
        count,
        color: iconicColors[level % iconicColors.length],
      })
    );
  const specialTalentData: CombatLabDonutDatum[] = [
    {
      key: "special-talent",
      name: labels.specialTalent,
      count: specialTalent,
      color: "#2563eb",
    },
    {
      key: "no-special-talent",
      name: labels.noSpecialTalent,
      count: noSpecialTalent,
      color: "#a1a1aa",
    },
  ].filter((item) => item.count > 0);

  return {
    chartData,
    iconicData,
    nonLegendaryCount,
    series,
    specialTalentData,
    totals: { observations, legendary, specialTalent },
  };
}

function formatArmamentValue(value: number, decimalFormatter: Intl.NumberFormat, isPercent = true) {
  return isPercent ? `${decimalFormatter.format(value * 100)}%` : decimalFormatter.format(value);
}

function trendBucketDays(rangeKey: CombatLabPreviewRangeKey) {
  return rangeKey === "1y" ? 14 : rangeKey === "6m" ? 7 : 1;
}

function trendDateFormatter(locale: string) {
  return new Intl.DateTimeFormat(locale, {
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });
}

function createCompactFormatter(locale: string) {
  return new Intl.NumberFormat(locale, {
    notation: "compact",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

function createDecimalFormatter(locale: string) {
  return new Intl.NumberFormat(locale, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

function createIntegerFormatter(locale: string) {
  return new Intl.NumberFormat(locale, { maximumFractionDigits: 0 });
}
