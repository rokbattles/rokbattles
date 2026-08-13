"use client";

import { useExtracted, useLocale } from "next-intl";
import { useMemo } from "react";
import {
  CartesianGrid,
  Line,
  LineChart,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  type TooltipContentProps,
  XAxis,
  YAxis,
} from "recharts";
import { CombatLabUsageTooltip } from "@/components/combat-lab/combat-lab-donut";
import { CombatLabEmptyState } from "@/components/combat-lab/combat-lab-empty-state";
import { Subheading } from "@/components/ui/heading";
import type {
  CombatLabPreviewLoadouts,
  CombatLabPreviewRangeKey,
  CombatLabPreviewSkillUsage,
} from "@/lib/combat-lab/preview-types";

const DAY_MS = 24 * 60 * 60 * 1_000;

const skillPairs = Array.from({ length: 25 }, (_, index) => {
  const first = Math.floor(index / 5) + 1;
  const second = (index % 5) + 1;
  return `${first}${second}`;
});

const axisTickIndexes = [0, 6, 12, 18, 24] as const;

type SkillRole = "primary" | "secondary";

type SkillHeatmapPoint = {
  count: number;
  skills: string;
  x: number;
};

type SkillPreview = {
  expertisePoints: SkillExpertiseChartPoint[];
  heatmap: SkillHeatmapPoint[];
  heatmapTotal: number;
  maxValue: number;
  role: SkillRole;
};

type SkillExpertiseChartPoint = {
  date: string;
  expertised: number;
  notExpertised: number;
};

type SkillExpertiseBucket = {
  bucketStartMs: number;
  expertisedCount: number;
  notExpertisedCount: number;
  sampleSize: number;
};

export function CombatLabSkills({
  primaryCommanderName,
  rangeKey,
  secondaryCommanderName,
  skills,
}: {
  primaryCommanderName: string;
  rangeKey: CombatLabPreviewRangeKey;
  secondaryCommanderName: string;
  skills?: CombatLabPreviewLoadouts["skills"] | null;
}) {
  const locale = useLocale();
  const skillPreviews = useMemo(
    () => [
      createSkillPreview("primary", skills?.primary, rangeKey, locale),
      createSkillPreview("secondary", skills?.secondary, rangeKey, locale),
    ],
    [locale, rangeKey, skills]
  );

  return (
    <>
      {skillPreviews.map((preview) => (
        <SkillCard
          commanderName={preview.role === "primary" ? primaryCommanderName : secondaryCommanderName}
          key={preview.role}
          preview={preview}
        />
      ))}
    </>
  );
}

function SkillCard({ commanderName, preview }: { commanderName: string; preview: SkillPreview }) {
  const t = useExtracted();
  const locale = useLocale();
  const decimalFormatter = useMemo(
    () =>
      new Intl.NumberFormat(locale, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      }),
    [locale]
  );
  const integerFormatter = useMemo(
    () => new Intl.NumberFormat(locale, { maximumFractionDigits: 0 }),
    [locale]
  );
  const accent = preview.role === "primary" ? "#2563eb" : "#7c3aed";
  const tooltipLabels = { count: t("Count"), usage: t("Usage") };

  return (
    <article className="min-w-0 rounded-md border border-zinc-950/10 bg-white p-5 dark:border-white/10 dark:bg-zinc-900 sm:p-6">
      <Subheading level={3} className="!text-lg/7">
        {t("Skills")}: {commanderName}
      </Subheading>
      <Subheading level={4} className="mt-3">
        {t("Skills usage")}
      </Subheading>

      {preview.heatmapTotal === 0 ? (
        <CombatLabEmptyState className="mt-3" />
      ) : (
        <>
          <div className="mt-3 grid grid-cols-[auto_auto_minmax(0,1fr)] gap-x-2">
            <div className="col-start-3 text-center font-medium text-xs text-zinc-500 dark:text-zinc-400">
              {t("Skills 1 & 2")}
            </div>
            <div className="col-start-3 mt-1.5 mb-1.5 grid grid-cols-5 font-medium text-[0.625rem]/4 text-zinc-500 tabular-nums dark:text-zinc-400">
              {axisTickIndexes.map((index) => (
                <span className="text-center" key={index}>
                  {skillPairs[index]}
                </span>
              ))}
            </div>
            <div className="col-start-1 row-start-3 flex items-center [writing-mode:vertical-rl] rotate-180 justify-center font-medium text-xs text-zinc-500 dark:text-zinc-400">
              {t("Skills 3 & 4")}
            </div>
            <div className="col-start-2 row-start-3 flex flex-col justify-between py-0.5 font-medium text-[0.625rem]/4 text-zinc-500 tabular-nums dark:text-zinc-400">
              {axisTickIndexes.map((index) => (
                <span key={index}>{skillPairs[index]}</span>
              ))}
            </div>
            <div
              aria-label={t("Heatmap of four-skill level combinations from 1111 to 5555")}
              className="col-start-3 row-start-3 grid aspect-square grid-cols-[repeat(25,minmax(0,1fr))] gap-px rounded-sm bg-zinc-950/10 p-px shadow-inner dark:bg-white/10"
              role="img"
            >
              {preview.heatmap.map((point) => (
                <HeatmapCell
                  accent={accent}
                  decimalFormatter={decimalFormatter}
                  integerFormatter={integerFormatter}
                  key={point.skills}
                  maxValue={preview.maxValue}
                  point={point}
                  tooltipLabels={tooltipLabels}
                  total={preview.heatmapTotal}
                />
              ))}
            </div>
          </div>

          <div className="mt-3 flex items-center justify-end gap-1.5 font-medium text-xs text-zinc-600 dark:text-zinc-300">
            <span>{t("Fewer")}</span>
            {[0.16, 0.32, 0.52, 0.72, 0.94].map((opacity) => (
              <span
                className="size-2.5 rounded-[2px]"
                key={opacity}
                style={{ backgroundColor: toRgba(accent, opacity) }}
              />
            ))}
            <span>{t("More")}</span>
          </div>

          <div className="mt-7">
            <Subheading level={4}>{t("Expertise status")}</Subheading>
            <div className="mt-3 flex flex-wrap gap-x-4 gap-y-2 text-xs font-medium">
              <ChartLegend color={accent} label={t("Expertised")} />
              <ChartLegend color="#a1a1aa" label={t("Not expertised")} />
            </div>
            {preview.expertisePoints.length === 0 ? (
              <CombatLabEmptyState className="mt-4 min-h-64" />
            ) : (
              <div className="mt-4 h-72">
                <ResponsiveContainer minWidth={0}>
                  <LineChart
                    data={preview.expertisePoints}
                    margin={{ top: 4, right: 4, bottom: 4, left: 4 }}
                  >
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
                      tickFormatter={(value) => `${Number(value)}%`}
                      tickLine={false}
                      width={42}
                    />
                    <RechartsTooltip
                      content={(props) => (
                        <ExpertiseTooltip
                          {...props}
                          decimalFormatter={decimalFormatter}
                          labels={{
                            expertised: t("Expertised"),
                            notExpertised: t("Not expertised"),
                          }}
                        />
                      )}
                    />
                    <Line
                      dataKey="expertised"
                      dot={false}
                      isAnimationActive={false}
                      stroke={accent}
                      strokeWidth={2.5}
                      type="monotone"
                    />
                    <Line
                      dataKey="notExpertised"
                      dot={false}
                      isAnimationActive={false}
                      stroke="#a1a1aa"
                      strokeWidth={2.25}
                      type="monotone"
                    />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            )}
          </div>
        </>
      )}
    </article>
  );
}

function HeatmapCell({
  accent,
  decimalFormatter,
  integerFormatter,
  maxValue,
  point,
  tooltipLabels,
  total,
}: {
  accent: string;
  decimalFormatter: Intl.NumberFormat;
  integerFormatter: Intl.NumberFormat;
  maxValue: number;
  point: SkillHeatmapPoint;
  tooltipLabels: { count: string; usage: string };
  total: number;
}) {
  const intensity =
    point.count === 0 ? 0.08 : 0.2 + 0.74 * (Math.log1p(point.count) / Math.log1p(maxValue));
  let horizontalPosition = "left-1/2 -translate-x-1/2";
  if (point.x < 5) horizontalPosition = "left-0";
  if (point.x > 19) horizontalPosition = "right-0";

  return (
    <div
      className="group/cell relative min-h-1 rounded-[1px] outline-none ring-inset hover:z-20 hover:ring-1 hover:ring-white/90"
      style={{ backgroundColor: toRgba(accent, intensity) }}
    >
      <div
        className={`pointer-events-none absolute bottom-full mb-1.5 hidden w-max group-hover/cell:block ${horizontalPosition}`}
      >
        <CombatLabUsageTooltip
          color={accent}
          count={point.count}
          decimalFormatter={decimalFormatter}
          integerFormatter={integerFormatter}
          labels={tooltipLabels}
          name={point.skills}
          total={total}
        />
      </div>
    </div>
  );
}

function ChartLegend({ color, label }: { color: string; label: string }) {
  return (
    <span className="inline-flex items-center gap-1.5 text-zinc-600 dark:text-zinc-300">
      <span className="size-2 rounded-full" style={{ backgroundColor: color }} />
      {label}
    </span>
  );
}

function ExpertiseTooltip({
  active,
  decimalFormatter,
  label,
  labels,
  payload,
}: TooltipContentProps & {
  decimalFormatter: Intl.NumberFormat;
  labels: {
    expertised: string;
    notExpertised: string;
  };
}) {
  const point = payload[0]?.payload as SkillExpertiseChartPoint | undefined;
  if (!(active && point)) return null;

  const statuses = [
    {
      color: payload.find((entry) => entry.dataKey === "expertised")?.color ?? "#2563eb",
      name: labels.expertised,
      usage: point.expertised,
    },
    {
      color: payload.find((entry) => entry.dataKey === "notExpertised")?.color ?? "#a1a1aa",
      name: labels.notExpertised,
      usage: point.notExpertised,
    },
  ];

  return (
    <div
      className="min-w-44 rounded-md border border-zinc-950/10 bg-white px-3 py-2 text-xs text-zinc-950 dark:border-white/10 dark:bg-zinc-900 dark:text-white"
      data-chart-tooltip=""
    >
      <div className="mb-2 text-zinc-500 dark:text-zinc-400">{label}</div>
      <div className="space-y-1">
        {statuses.map((status) => (
          <div className="flex items-center justify-between gap-4" key={status.name}>
            <span className="inline-flex items-center gap-1.5">
              <span className="size-2 rounded-full" style={{ backgroundColor: status.color }} />
              {status.name}
            </span>
            <span className="font-medium tabular-nums">
              {decimalFormatter.format(status.usage)}%
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

function createSkillPreview(
  role: SkillRole,
  usage: CombatLabPreviewSkillUsage | undefined,
  rangeKey: CombatLabPreviewRangeKey,
  locale: string
): SkillPreview {
  const counts = new Map(usage?.builds.map((build) => [build.skills, build.count]) ?? []);
  const heatmap = skillPairs.flatMap((rowPair) =>
    skillPairs.map((columnPair, x) => ({
      count: counts.get(`${columnPair}${rowPair}`) ?? 0,
      skills: `${columnPair}${rowPair}`,
      x,
    }))
  );
  const heatmapTotal = heatmap.reduce((sum, point) => sum + point.count, 0);

  return {
    expertisePoints: aggregateExpertisePoints(usage?.expertisePoints ?? [], rangeKey, locale),
    heatmap,
    heatmapTotal,
    maxValue: Math.max(1, ...heatmap.map((point) => point.count)),
    role,
  };
}

function aggregateExpertisePoints(
  points: CombatLabPreviewSkillUsage["expertisePoints"],
  rangeKey: CombatLabPreviewRangeKey,
  locale: string
): SkillExpertiseChartPoint[] {
  const bucketMs = (rangeKey === "1y" ? 14 : rangeKey === "6m" ? 7 : 1) * DAY_MS;
  const grouped = new Map<number, SkillExpertiseBucket>();

  for (const point of points) {
    const bucketStartMs = Math.floor(point.bucketStartMs / bucketMs) * bucketMs;
    const bucket = grouped.get(bucketStartMs) ?? {
      bucketStartMs,
      expertisedCount: 0,
      notExpertisedCount: 0,
      sampleSize: 0,
    };
    bucket.sampleSize += point.sampleSize;
    bucket.expertisedCount += point.expertisedCount;
    bucket.notExpertisedCount += point.notExpertisedCount;
    grouped.set(bucketStartMs, bucket);
  }

  const dateFormatter = new Intl.DateTimeFormat(locale, {
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });
  return Array.from(grouped.values())
    .filter((point) => point.sampleSize > 0)
    .sort((left, right) => left.bucketStartMs - right.bucketStartMs)
    .map((point) => ({
      date: dateFormatter.format(point.bucketStartMs),
      expertised: percent(point.expertisedCount, point.sampleSize),
      notExpertised: percent(point.notExpertisedCount, point.sampleSize),
    }));
}

function toRgba(hex: string, opacity: number) {
  const red = Number.parseInt(hex.slice(1, 3), 16);
  const green = Number.parseInt(hex.slice(3, 5), 16);
  const blue = Number.parseInt(hex.slice(5, 7), 16);
  return `rgba(${red}, ${green}, ${blue}, ${opacity})`;
}

function percent(count: number, total: number) {
  return total > 0 ? (count / total) * 100 : 0;
}
