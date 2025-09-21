"use client";

import clsx from "clsx";
import { useTranslations } from "next-intl";
import { useMemo, useState } from "react";
import type { TooltipProps } from "recharts";
import {
  Area,
  Bar,
  BarChart,
  CartesianGrid,
  ComposedChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { formatNumber } from "@/lib/formatNumber";
import type { BattleResultsSummary, BattleResultsTimelineEntry } from "@/lib/types/reports";
import { formatUTCShort } from "@/lib/utc";

type BattleResultsTimelineChartProps = {
  summary: BattleResultsSummary;
  locale: string;
};

type MetricKey = keyof BattleResultsTimelineEntry;

type MetricLabelKey = "dead" | "severelyWounded" | "slightlyWounded" | "killPoints";

type MetricConfig = {
  key: MetricKey;
  enemyKey: MetricKey;
  labelKey: MetricLabelKey;
  color: string;
  enemyColor: string;
};

const METRICS: readonly MetricConfig[] = [
  {
    key: "death",
    enemyKey: "enemy_death",
    labelKey: "dead",
    color: "#f97316",
    enemyColor: "#fb7185",
  },
  {
    key: "severely_wounded",
    enemyKey: "enemy_severely_wounded",
    labelKey: "severelyWounded",
    color: "#fbbf24",
    enemyColor: "#f9a8d4",
  },
  {
    key: "wounded",
    enemyKey: "enemy_wounded",
    labelKey: "slightlyWounded",
    color: "#38bdf8",
    enemyColor: "#818cf8",
  },
  {
    key: "kill_score",
    enemyKey: "enemy_kill_score",
    labelKey: "killPoints",
    color: "#34d399",
    enemyColor: "#60a5fa",
  },
];

type TimelineDatum = {
  timestamp: number;
} & Record<string, number>;

type SideFilter = "self" | "enemy" | "both";

type ChartTooltipValue = {
  dataKey?: string;
  value?: number;
  payload?: Record<string, unknown>;
};

type TimelineTooltipProps = TooltipProps<number, string> & {
  payload?: ChartTooltipValue[];
  label?: number | string;
};

type TotalsTooltipProps = TooltipProps<number, string> & {
  payload?: ChartTooltipValue[];
  label?: number | string;
};

function formatTimestampLabel(value: number, locale: string) {
  const safeValue = Number.isFinite(value) ? value : 0;
  const date = new Date(safeValue);
  try {
    return new Intl.DateTimeFormat(locale, {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
      timeZone: "UTC",
    }).format(date);
  } catch {
    return date.toUTCString();
  }
}

export function BattleResultsTimelineChart({ summary, locale }: BattleResultsTimelineChartProps) {
  const t = useTranslations("battle.metrics");

  const [activeMetrics, setActiveMetrics] = useState<Set<MetricKey>>(
    () => new Set(METRICS.map((metric) => metric.key))
  );
  const [sideFilter, setSideFilter] = useState<SideFilter>("both");

  const timelineData = useMemo(() => {
    const entries = summary.timeline ?? [];
    return entries
      .filter((entry) => typeof entry?.start_date === "number" && Number(entry.start_date) > 0)
      .map((entry) => {
        const startDate = (entry.start_date ?? 0) * 1000;
        const base: TimelineDatum = {
          timestamp: startDate,
        };
        for (const metric of METRICS) {
          const selfValue = Number(entry?.[metric.key] ?? 0);
          const enemyValue = Number(entry?.[metric.enemyKey] ?? 0);
          base[`self_${metric.key}`] = Number.isFinite(selfValue) ? selfValue : 0;
          base[`enemy_${metric.key}`] = Number.isFinite(enemyValue) ? enemyValue : 0;
        }
        return base;
      })
      .sort((a, b) => a.timestamp - b.timestamp);
  }, [summary.timeline]);

  const totalsData = useMemo(() => {
    const totals = summary.total ?? {};
    return METRICS.map((metric) => {
      const selfValue = Number((totals as Record<string, unknown>)[metric.key] ?? 0);
      const enemyValue = Number((totals as Record<string, unknown>)[metric.enemyKey] ?? 0);
      return {
        metricKey: metric.key,
        label: t(metric.labelKey),
        self: Number.isFinite(selfValue) ? selfValue : 0,
        enemy: Number.isFinite(enemyValue) ? enemyValue : 0,
      };
    }).filter((item) => item.self > 0 || item.enemy > 0);
  }, [summary.total, t]);

  const showFriendly = sideFilter === "self" || sideFilter === "both";
  const showEnemy = sideFilter === "enemy" || sideFilter === "both";

  if (timelineData.length < 2) {
    return null;
  }

  const handleMetricToggle = (metricKey: MetricKey) => {
    setActiveMetrics((prev) => {
      const next = new Set(prev);
      if (next.has(metricKey)) {
        if (next.size === 1) {
          return prev;
        }
        next.delete(metricKey);
      } else {
        next.add(metricKey);
      }
      if (next.size === prev.size && [...next].every((key) => prev.has(key))) {
        return prev;
      }
      return next;
    });
  };

  const timelineTooltip = (props: TimelineTooltipProps) => {
    if (!props.active || !props.payload?.length) return null;
    const datum = props.payload?.[0]?.payload as Partial<TimelineDatum> | undefined;
    const timestamp =
      typeof datum?.timestamp === "number"
        ? datum.timestamp
        : typeof props.label === "number"
          ? props.label
          : 0;
    const labelText =
      timestamp > 0
        ? (formatUTCShort(Math.floor(timestamp / 1000), undefined) ??
          formatTimestampLabel(timestamp, locale))
        : "\u2014";
    const groups = props.payload
      .map((item) => {
        if (!item?.dataKey || typeof item.value !== "number") return null;
        const metric = METRICS.find(
          (entry) => item.dataKey === `self_${entry.key}` || item.dataKey === `enemy_${entry.key}`
        );
        if (!metric) return null;
        const side = item.dataKey.startsWith("enemy_") ? "enemy" : "self";
        if ((side === "self" && !showFriendly) || (side === "enemy" && !showEnemy)) return null;
        if (!activeMetrics.has(metric.key)) return null;
        return {
          id: `${side}_${metric.key}`,
          side,
          metric,
          value: Math.max(0, item.value),
        };
      })
      .filter(
        (
          value
        ): value is { id: string; side: "self" | "enemy"; metric: MetricConfig; value: number } =>
          value !== null
      )
      .filter((value) => value.value > 0)
      .sort((a, b) => {
        const metricOrder =
          METRICS.findIndex((item) => item.key === a.metric.key) -
          METRICS.findIndex((item) => item.key === b.metric.key);
        if (metricOrder !== 0) return metricOrder;
        return a.side.localeCompare(b.side);
      });

    if (groups.length === 0) return null;

    return (
      <div className="min-w-[220px] rounded-md border border-white/10 bg-zinc-900/95 p-3 text-sm shadow-lg">
        <div className="mb-2 text-xs font-medium text-zinc-300">{labelText}</div>
        <div className="space-y-1 text-xs">
          {groups.map((group) => (
            <div key={group.id} className="flex items-center justify-between gap-2">
              <span className="flex items-center gap-2 text-zinc-300">
                <span
                  className="h-2 w-2 rounded-full"
                  style={{
                    backgroundColor:
                      group.side === "self" ? group.metric.color : group.metric.enemyColor,
                  }}
                />
                <span>
                  {t(group.metric.labelKey)}
                  <span className="ml-1 text-zinc-500">
                    ({group.side === "self" ? "Self" : "Enemy"})
                  </span>
                </span>
              </span>
              <span className="font-medium text-zinc-100">{formatNumber(group.value, locale)}</span>
            </div>
          ))}
        </div>
      </div>
    );
  };

  const totalsTooltip = (props: TotalsTooltipProps) => {
    if (!props.active || !props.payload?.length) return null;
    const metricKey = props.payload[0]?.payload?.metricKey as MetricKey | undefined;
    const metric = METRICS.find((item) => item.key === metricKey);
    if (!metric) return null;
    const friendlyValue = props.payload.find((item) => item.dataKey === "self")?.value as
      | number
      | undefined;
    const enemyValue = props.payload.find((item) => item.dataKey === "enemy")?.value as
      | number
      | undefined;
    if ((friendlyValue ?? 0) <= 0 && (enemyValue ?? 0) <= 0) return null;

    return (
      <div className="min-w-[200px] rounded-md border border-white/10 bg-zinc-900/95 p-3 text-sm shadow-lg">
        <div className="mb-2 text-xs font-semibold text-zinc-300">{t(metric.labelKey)}</div>
        <div className="space-y-1 text-xs">
          {showFriendly && friendlyValue && friendlyValue > 0 ? (
            <div className="flex items-center justify-between gap-2">
              <span className="flex items-center gap-2">
                <span className="h-2 w-2 rounded-full" style={{ backgroundColor: metric.color }} />
                <span className="text-zinc-300">Self</span>
              </span>
              <span className="font-medium text-zinc-100">
                {formatNumber(friendlyValue, locale)}
              </span>
            </div>
          ) : null}
          {showEnemy && enemyValue && enemyValue > 0 ? (
            <div className="flex items-center justify-between gap-2">
              <span className="flex items-center gap-2">
                <span
                  className="h-2 w-2 rounded-full"
                  style={{ backgroundColor: metric.enemyColor }}
                />
                <span className="text-zinc-300">Enemy</span>
              </span>
              <span className="font-medium text-zinc-100">{formatNumber(enemyValue, locale)}</span>
            </div>
          ) : null}
        </div>
      </div>
    );
  };

  const totalsChartHeight = Math.max(140, totalsData.length * 48);

  return (
    <section className="space-y-4 rounded-lg bg-zinc-800/50 p-3 ring-1 ring-white/5">
      <div className="space-y-3">
        <h3 className="text-sm font-semibold text-zinc-100">Battle timeline</h3>
        <div className="flex flex-wrap gap-2 text-xs">
          {METRICS.map((metric) => {
            const isActive = activeMetrics.has(metric.key);
            return (
              <button
                key={metric.key}
                type="button"
                onClick={() => handleMetricToggle(metric.key)}
                className={clsx(
                  "flex items-center gap-2 rounded-md px-2 py-1 text-xs font-medium transition",
                  isActive
                    ? "bg-zinc-900 text-zinc-100 ring-1 ring-white/20"
                    : "bg-zinc-900/40 text-zinc-500 hover:bg-zinc-900/60 hover:text-zinc-200"
                )}
                aria-pressed={isActive}
              >
                <span
                  className="h-2 w-2 rounded-full"
                  style={{
                    backgroundColor: isActive ? metric.color : `${metric.color}80`,
                  }}
                />
                {t(metric.labelKey)}
              </button>
            );
          })}
        </div>
        <div className="flex flex-wrap gap-2 text-xs">
          {(
            [
              { value: "self" as SideFilter, label: "Self" },
              { value: "enemy" as SideFilter, label: "Enemy" },
              { value: "both" as SideFilter, label: "Both" },
            ] as const
          ).map((option) => {
            const isActive = sideFilter === option.value;
            return (
              <button
                key={option.value}
                type="button"
                onClick={() => setSideFilter(option.value)}
                className={clsx(
                  "rounded-md px-2 py-1 text-xs font-medium transition",
                  isActive
                    ? "bg-blue-500/20 text-blue-200 ring-1 ring-blue-400/40"
                    : "bg-zinc-900/40 text-zinc-500 hover:bg-zinc-900/60 hover:text-zinc-200"
                )}
                aria-pressed={isActive}
              >
                {option.label}
              </button>
            );
          })}
        </div>
      </div>

      <div className="h-[320px] w-full">
        <ResponsiveContainer width="100%" height="100%">
          <ComposedChart data={timelineData} margin={{ top: 10, right: 16, bottom: 0, left: 0 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.08)" />
            <XAxis
              dataKey="timestamp"
              type="number"
              domain={["dataMin", "dataMax"]}
              scale="time"
              tickFormatter={(value) => formatTimestampLabel(Number(value), locale)}
              tick={{ fill: "#a1a1aa", fontSize: 11 }}
              axisLine={{ stroke: "rgba(255,255,255,0.1)" }}
              tickLine={false}
            />
            <YAxis
              tickFormatter={(value) => formatNumber(Number(value), locale)}
              tick={{ fill: "#a1a1aa", fontSize: 11 }}
              axisLine={{ stroke: "rgba(255,255,255,0.1)" }}
              tickLine={false}
            />
            <Tooltip
              content={timelineTooltip}
              cursor={{ stroke: "rgba(148,163,184,0.4)", strokeWidth: 1 }}
            />
            {showFriendly
              ? METRICS.map((metric) => (
                  <Area
                    key={`self-${metric.key}`}
                    type="monotone"
                    dataKey={`self_${metric.key}`}
                    stroke={metric.color}
                    fill={metric.color}
                    fillOpacity={activeMetrics.has(metric.key) ? 0.25 : 0}
                    strokeOpacity={activeMetrics.has(metric.key) ? 1 : 0}
                    strokeWidth={2}
                    stackId="self"
                    hide={!activeMetrics.has(metric.key)}
                  />
                ))
              : null}
            {showEnemy
              ? METRICS.map((metric) => (
                  <Area
                    key={`enemy-${metric.key}`}
                    type="monotone"
                    dataKey={`enemy_${metric.key}`}
                    stroke={metric.enemyColor}
                    fill={metric.enemyColor}
                    fillOpacity={activeMetrics.has(metric.key) ? 0.18 : 0}
                    strokeOpacity={activeMetrics.has(metric.key) ? 0.9 : 0}
                    strokeWidth={2}
                    stackId="enemy"
                    hide={!activeMetrics.has(metric.key)}
                  />
                ))
              : null}
          </ComposedChart>
        </ResponsiveContainer>
      </div>

      {totalsData.length > 0 ? (
        <div>
          <h4 className="text-sm font-semibold text-zinc-100">Totals comparison</h4>
          <div className="mt-3" style={{ height: totalsChartHeight }}>
            <ResponsiveContainer width="100%" height="100%">
              <BarChart
                data={totalsData}
                layout="vertical"
                margin={{ top: 8, right: 16, bottom: 0, left: 0 }}
                barCategoryGap={12}
              >
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.08)" />
                <XAxis
                  type="number"
                  tickFormatter={(value) => formatNumber(Number(value), locale)}
                  tick={{ fill: "#a1a1aa", fontSize: 11 }}
                  axisLine={{ stroke: "rgba(255,255,255,0.1)" }}
                  tickLine={false}
                />
                <YAxis
                  dataKey="label"
                  type="category"
                  width={120}
                  tick={{ fill: "#a1a1aa", fontSize: 11 }}
                  axisLine={{ stroke: "rgba(255,255,255,0.1)" }}
                  tickLine={false}
                />
                <Tooltip content={totalsTooltip} cursor={{ fill: "rgba(148,163,184,0.08)" }} />
                {showFriendly ? <Bar dataKey="self" fill="#0ea5e9" radius={[4, 4, 4, 4]} /> : null}
                {showEnemy ? <Bar dataKey="enemy" fill="#f43f5e" radius={[4, 4, 4, 4]} /> : null}
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>
      ) : null}
    </section>
  );
}
