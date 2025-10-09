"use client";

import { useMemo, useState } from "react";
import {
  Area,
  CartesianGrid,
  ComposedChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { cn } from "@/lib/cn";
import type { BattleResultsSummary, BattleResultsTimelineEntry } from "@/lib/types/report";

type MetricKey = keyof Pick<
  BattleResultsTimelineEntry,
  "death" | "severelyWounded" | "wounded" | "killScore"
>;

type EnemyMetricKey = keyof Pick<
  BattleResultsTimelineEntry,
  "enemyDeath" | "enemySeverelyWounded" | "enemyWounded" | "enemyKillScore"
>;

type MetricConfig = {
  key: MetricKey;
  enemyKey: EnemyMetricKey;
  label: string;
  color: string;
  enemyColor: string;
};

const METRICS: readonly MetricConfig[] = [
  {
    key: "death",
    enemyKey: "enemyDeath",
    label: "Dead",
    color: "#f97316",
    enemyColor: "#fb7185",
  },
  {
    key: "severelyWounded",
    enemyKey: "enemySeverelyWounded",
    label: "Severely wounded",
    color: "#fbbf24",
    enemyColor: "#f9a8d4",
  },
  {
    key: "wounded",
    enemyKey: "enemyWounded",
    label: "Slightly wounded",
    color: "#38bdf8",
    enemyColor: "#818cf8",
  },
  {
    key: "killScore",
    enemyKey: "enemyKillScore",
    label: "Kill points",
    color: "#34d399",
    enemyColor: "#60a5fa",
  },
] as const;

type SeriesKey = MetricConfig["key"] | MetricConfig["enemyKey"];

const SERIES_KEYS: readonly SeriesKey[] = METRICS.flatMap((metric) => [
  metric.key,
  metric.enemyKey,
]) as SeriesKey[];

type TimelineDatum = {
  timestamp: number;
} & Partial<Record<SeriesKey, number>>;

const compactFormatter = new Intl.NumberFormat("en-US", {
  notation: "compact",
  maximumFractionDigits: 1,
});

const axisFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

type SideFilter = "both" | "self" | "enemy";

const sideOptions: Array<{ id: SideFilter; label: string }> = [
  { id: "both", label: "Both" },
  { id: "self", label: "Self" },
  { id: "enemy", label: "Enemy" },
];

export function ReportTimelineChart({ summary }: { summary: BattleResultsSummary }) {
  const [activeMetrics, setActiveMetrics] = useState<Set<MetricKey>>(
    () => new Set(METRICS.map((metric) => metric.key))
  );
  const [sideFilter, setSideFilter] = useState<SideFilter>("both");

  const timeline = useMemo(() => buildTimelineData(summary.timeline ?? []), [summary.timeline]);

  const visibleMetrics = useMemo(
    () => METRICS.filter((metric) => activeMetrics.has(metric.key)),
    [activeMetrics]
  );

  const toggleMetric = (metric: MetricKey) => {
    setActiveMetrics((prev) => {
      const next = new Set(prev);
      if (next.has(metric)) {
        next.delete(metric);
      } else {
        next.add(metric);
      }
      return next.size === 0 ? prev : next;
    });
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        {METRICS.map((metric) => {
          const isActive = activeMetrics.has(metric.key);
          return (
            <button
              key={metric.key}
              type="button"
              onClick={() => toggleMetric(metric.key)}
              className={cn(
                "rounded-full px-3 py-1 text-xs font-medium transition-colors",
                "border border-zinc-950/15 dark:border-white/15",
                isActive
                  ? "bg-zinc-950 text-white shadow-sm dark:bg-white/90 dark:text-zinc-950"
                  : "bg-white text-zinc-600 dark:bg-zinc-900 dark:text-zinc-300"
              )}
            >
              {metric.label}
            </button>
          );
        })}
        <div className="ml-auto flex items-center gap-1 rounded-full border border-zinc-950/15 bg-zinc-50 p-0.5 dark:border-white/15 dark:bg-white/10">
          {sideOptions.map((option) => {
            const isActive = option.id === sideFilter;
            return (
              <button
                key={option.id}
                type="button"
                onClick={() => setSideFilter(option.id)}
                className={cn(
                  "rounded-full px-2.5 py-1 text-xs font-medium transition-colors",
                  isActive
                    ? "bg-white text-zinc-950 shadow-sm dark:bg-zinc-900 dark:text-white"
                    : "text-zinc-600 dark:text-zinc-300"
                )}
              >
                {option.label}
              </button>
            );
          })}
        </div>
      </div>
      <div className="h-72 w-full">
        <ResponsiveContainer>
          <ComposedChart data={timeline} margin={{ top: 10, right: 20, left: 0, bottom: 0 }}>
            <CartesianGrid stroke="#e5e7eb" strokeDasharray="3 3" vertical={false} />
            <XAxis
              dataKey="timestamp"
              tick={{ fontSize: 12, fill: "#6b7280" }}
              tickMargin={8}
              tickFormatter={formatTime}
            />
            <YAxis
              tick={{ fontSize: 12, fill: "#6b7280" }}
              tickMargin={8}
              tickFormatter={(value: number) => axisFormatter.format(value)}
            />
            <Tooltip content={<TimelineTooltip />} />
            {visibleMetrics.map((metric) => (
              <MetricSeries key={metric.key} metric={metric} sideFilter={sideFilter} />
            ))}
          </ComposedChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

function MetricSeries({ metric, sideFilter }: { metric: MetricConfig; sideFilter: SideFilter }) {
  return (
    <>
      {(sideFilter === "both" || sideFilter === "self") && (
        <Area
          type="monotone"
          dataKey={metric.key}
          stroke={metric.color}
          fill={metric.color}
          fillOpacity={0.18}
          strokeWidth={2}
          connectNulls
          animationDuration={300}
          dot={false}
          isAnimationActive={false}
        />
      )}
      {(sideFilter === "both" || sideFilter === "enemy") && (
        <Area
          type="monotone"
          dataKey={metric.enemyKey}
          stroke={metric.enemyColor}
          fill={metric.enemyColor}
          fillOpacity={0.18}
          strokeWidth={2}
          connectNulls
          animationDuration={300}
          dot={false}
          isAnimationActive={false}
        />
      )}
    </>
  );
}

type TooltipPayloadItem = {
  dataKey?: string;
  value?: number;
};

function TimelineTooltip({
  active,
  label,
  payload,
}: {
  active?: boolean;
  label?: number;
  payload?: TooltipPayloadItem[];
}) {
  if (!active || !payload || payload.length === 0 || typeof label !== "number") {
    return null;
  }

  const date = new Date(label);

  return (
    <div className="rounded-lg border border-zinc-950/10 bg-white px-4 py-3 text-xs shadow-lg dark:border-white/10 dark:bg-zinc-900">
      <div className="font-semibold text-zinc-700 dark:text-zinc-200">{formatFullTime(date)}</div>
      <ul className="mt-3 space-y-1.5">
        {payload.map((entry) => {
          if (typeof entry.value !== "number" || typeof entry.dataKey !== "string") {
            return null;
          }
          const descriptor = resolveSeriesDescriptor(entry.dataKey);
          if (!descriptor) {
            return null;
          }
          return (
            <li key={`${entry.dataKey}-${descriptor.label}`} className="flex items-center gap-2">
              <span
                className="inline-block h-2 w-2 rounded-full"
                style={{ backgroundColor: descriptor.color }}
              />
              <span className="flex-1 text-zinc-600 dark:text-zinc-300">{descriptor.label}</span>
              <span className="font-mono text-zinc-800 dark:text-zinc-100">
                {compactFormatter.format(entry.value)}
              </span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

type SeriesDescriptor = {
  label: string;
  color: string;
};

function resolveSeriesDescriptor(dataKey: string): SeriesDescriptor | null {
  const metric = METRICS.find(
    (item) => item.key === dataKey || item.enemyKey === (dataKey as EnemyMetricKey)
  );

  if (!metric) {
    return null;
  }

  if (metric.key === dataKey) {
    return { label: `Self · ${metric.label}`, color: metric.color };
  }

  if (metric.enemyKey === dataKey) {
    return { label: `Enemy · ${metric.label}`, color: metric.enemyColor };
  }

  return null;
}

function formatTime(value: number) {
  const date = new Date(value);
  try {
    return new Intl.DateTimeFormat("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
      timeZone: "UTC",
    }).format(date);
  } catch {
    return date.toUTCString();
  }
}

function formatFullTime(date: Date) {
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  const hours = String(date.getUTCHours()).padStart(2, "0");
  const minutes = String(date.getUTCMinutes()).padStart(2, "0");
  return `UTC ${month}/${day} ${hours}:${minutes}`;
}

function buildTimelineData(
  entries: readonly BattleResultsTimelineEntry[] | undefined
): TimelineDatum[] {
  if (!entries || entries.length === 0) {
    return [];
  }

  const raw = entries
    .map((entry) => toTimelineDatum(entry))
    .filter((value): value is TimelineDatum => value !== null)
    .sort((a, b) => a.timestamp - b.timestamp);

  return bucketTimeline(raw, 200);
}

function toTimelineDatum(entry: BattleResultsTimelineEntry): TimelineDatum | null {
  const timestamp = toMillis(entry.startDate);
  if (timestamp == null) {
    return null;
  }

  const datum: TimelineDatum = {
    timestamp,
  };

  for (const metric of METRICS) {
    const selfValue = Number(entry[metric.key] ?? 0);
    if (Number.isFinite(selfValue) && selfValue !== 0) {
      datum[metric.key] = selfValue;
    }

    const enemyValue = Number(entry[metric.enemyKey] ?? 0);
    if (Number.isFinite(enemyValue) && enemyValue !== 0) {
      datum[metric.enemyKey] = enemyValue;
    }
  }

  return datum;
}

function bucketTimeline(data: TimelineDatum[], targetPoints: number): TimelineDatum[] {
  if (data.length <= targetPoints) {
    return data;
  }

  const minTs = data[0]?.timestamp ?? 0;
  const maxTs = data[data.length - 1]?.timestamp ?? minTs;

  if (minTs === maxTs) {
    return data;
  }

  const safeTarget = Math.max(1, targetPoints);
  const bucketSize = Math.max(1, Math.ceil((maxTs - minTs) / safeTarget));

  const buckets = new Map<number, TimelineDatum>();

  for (const item of data) {
    const bucketIndex = Math.floor((item.timestamp - minTs) / bucketSize);
    const bucketStart = minTs + bucketIndex * bucketSize;

    if (!buckets.has(bucketStart)) {
      buckets.set(bucketStart, { timestamp: bucketStart });
    }

    const bucket = buckets.get(bucketStart);
    if (!bucket) continue;

    for (const key of SERIES_KEYS) {
      const value = item[key];
      if (typeof value !== "number") continue;
      bucket[key] = (bucket[key] ?? 0) + value;
    }
  }

  return Array.from(buckets.values()).sort((a, b) => a.timestamp - b.timestamp);
}

function toMillis(value: number | null | undefined): number | null {
  if (!Number.isFinite(value ?? NaN)) {
    return null;
  }

  const numeric = Number(value);
  const absValue = Math.abs(numeric);
  const millis = absValue < 1_000_000_000_000 ? numeric * 1000 : numeric;
  return millis > 0 ? millis : null;
}
