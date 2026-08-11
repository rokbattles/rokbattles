"use client";

import {
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  type TooltipContentProps,
} from "recharts";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";

export type CombatLabDonutDatum = {
  color: string;
  count: number;
  key: string;
  name: string;
};

export function CombatLabDonut({
  ariaLabel,
  centerLabel,
  centerValue,
  data,
  decimalFormatter,
  emptyLabel,
  integerFormatter,
  title,
  tooltipLabels,
}: {
  ariaLabel: string;
  centerLabel: string;
  centerValue: string;
  data: CombatLabDonutDatum[];
  decimalFormatter: Intl.NumberFormat;
  emptyLabel: string;
  integerFormatter: Intl.NumberFormat;
  title: string;
  tooltipLabels: { count: string; usage: string };
}) {
  const total = data.reduce((sum, item) => sum + item.count, 0);

  return (
    <div className="min-w-0">
      <Subheading level={4} className="text-center">
        {title}
      </Subheading>
      {total > 0 ? (
        <div className="relative mt-2 h-44" role="img" aria-label={ariaLabel}>
          <ResponsiveContainer minWidth={0}>
            <PieChart>
              <Pie
                data={data}
                dataKey="count"
                innerRadius={50}
                isAnimationActive={false}
                nameKey="name"
                outerRadius={70}
                paddingAngle={2}
                stroke="transparent"
              >
                {data.map((item) => (
                  <Cell fill={item.color} key={item.key} />
                ))}
              </Pie>
              <Tooltip
                content={(props) => (
                  <CombatLabDonutTooltip
                    {...props}
                    decimalFormatter={decimalFormatter}
                    integerFormatter={integerFormatter}
                    labels={tooltipLabels}
                    total={total}
                  />
                )}
                wrapperStyle={{ zIndex: 20 }}
              />
            </PieChart>
          </ResponsiveContainer>
          <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
            <span className="font-semibold text-sm tabular-nums text-zinc-950 dark:text-white">
              {centerValue}
            </span>
            <span className="mt-0.5 text-[10px] text-zinc-500 dark:text-zinc-400">
              {centerLabel}
            </span>
          </div>
        </div>
      ) : (
        <Text className="mt-2 flex h-44 items-center justify-center text-center !text-xs/5">
          {emptyLabel}
        </Text>
      )}
      {total > 0 ? (
        <div className="mt-1 flex flex-wrap justify-center gap-x-3 gap-y-1.5 text-[11px] text-zinc-600 dark:text-zinc-300">
          {data.map((item) => (
            <span
              className="inline-flex min-w-0 max-w-full items-center gap-1.5"
              key={item.key}
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
      ) : null}
    </div>
  );
}

export function CombatLabUsageTooltip({
  color,
  count,
  decimalFormatter,
  integerFormatter,
  labels,
  name,
  total,
}: {
  color: string;
  count: number;
  decimalFormatter: Intl.NumberFormat;
  integerFormatter: Intl.NumberFormat;
  labels: { count: string; usage: string };
  name: string;
  total: number;
}) {
  return (
    <div
      className="rounded-md border border-zinc-950/10 bg-white px-3 py-2 text-xs text-zinc-950 dark:border-white/10 dark:bg-zinc-900 dark:text-white"
      data-chart-tooltip=""
    >
      <div className="flex items-center gap-1.5 font-medium">
        <span className="size-2 rounded-full" style={{ backgroundColor: color }} />
        {name}
      </div>
      <dl className="mt-2 grid grid-cols-[auto_auto] gap-x-5 gap-y-1 tabular-nums">
        <dt className="text-zinc-500 dark:text-zinc-400">{labels.usage}</dt>
        <dd className="text-right">{decimalFormatter.format((count / total) * 100)}%</dd>
        <dt className="text-zinc-500 dark:text-zinc-400">{labels.count}</dt>
        <dd className="text-right">{integerFormatter.format(count)}</dd>
      </dl>
    </div>
  );
}

function CombatLabDonutTooltip({
  active,
  decimalFormatter,
  integerFormatter,
  labels,
  payload,
  total,
}: TooltipContentProps & {
  decimalFormatter: Intl.NumberFormat;
  integerFormatter: Intl.NumberFormat;
  labels: { count: string; usage: string };
  total: number;
}) {
  const item = payload?.[0]?.payload as CombatLabDonutDatum | undefined;
  if (!(active && item)) {
    return null;
  }

  return (
    <CombatLabUsageTooltip
      color={item.color}
      count={item.count}
      decimalFormatter={decimalFormatter}
      integerFormatter={integerFormatter}
      labels={labels}
      name={item.name}
      total={total}
    />
  );
}
