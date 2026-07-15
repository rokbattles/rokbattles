"use client";

import { useExtracted, useLocale } from "next-intl";
import { useMemo } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  XAxis,
  YAxis,
} from "recharts";
import { getFormationName } from "@/hooks/use-formation-name";
import type { CombatLabFormation } from "@/lib/combat-lab/api";
import { formatPercent, numberFormatter } from "@/lib/combat-lab/format";

type FormationBreakdownProps = {
  formations: CombatLabFormation[];
};

type FormationCount = {
  key: string;
  formationId: number | null;
  count: number;
};

type FormationDatum = FormationCount & {
  color: string;
  label: string;
  percentage: number;
};

const WEDGE_FORMATION_ID = 2;
const WEDGE_FORMATION_IDS = new Set([WEDGE_FORMATION_ID, 19]);
const MAX_FORMATION_BARS = 5;
const VISIBLE_FORMATION_BARS_WITH_OTHER = MAX_FORMATION_BARS - 1;

const FORMATION_BAR_COLOR = "#2563eb";
const axisNumberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 1,
  notation: "compact",
});

export function FormationBreakdown({ formations }: FormationBreakdownProps) {
  const t = useExtracted();
  const locale = useLocale();
  const { entries, total } = useMemo(() => buildFormationCounts(formations), [formations]);
  const data = entries.map<FormationDatum>((entry) => ({
    ...entry,
    color: FORMATION_BAR_COLOR,
    label:
      entry.formationId === null
        ? t("Other")
        : entry.formationId === 0
          ? t("No formation")
          : (getFormationName(entry.formationId, locale) ??
            t("Formation {formation}", { formation: entry.formationId.toString() })),
    percentage: total === 0 ? 0 : (entry.count / total) * 100,
  }));

  return (
    <div className="col-span-2 flex h-full flex-col border-zinc-200/60 border-b pb-4 dark:border-white/10">
      <div className="space-y-1">
        <div className="font-semibold text-sm text-zinc-950 dark:text-white">{t("Formations")}</div>
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          {t("Formation usage across battle reports recorded for this pairing.")}
        </p>
      </div>
      {data.length === 0 ? (
        <p className="pt-4 text-sm text-zinc-500 dark:text-zinc-400">
          {t("No formation data available.")}
        </p>
      ) : (
        <div className="mt-4">
          <div className="h-48 w-full">
            <ResponsiveContainer>
              <BarChart
                barCategoryGap="8%"
                data={data}
                layout="vertical"
                margin={{ top: 4, right: 16, bottom: 4, left: 0 }}
              >
                <CartesianGrid horizontal={false} stroke="#e4e4e7" />
                <XAxis
                  allowDecimals={false}
                  axisLine={false}
                  type="number"
                  tick={{ fontSize: 11, fill: "#71717a" }}
                  tickFormatter={(value) => axisNumberFormatter.format(Number(value))}
                  tickLine={false}
                  tickMargin={8}
                />
                <YAxis
                  axisLine={false}
                  dataKey="label"
                  interval={0}
                  tick={<FormationAxisTick />}
                  tickLine={false}
                  type="category"
                  width={120}
                />
                <RechartsTooltip
                  cursor={{ fill: "rgba(39, 39, 42, 0.08)" }}
                  content={(props) => (
                    <FormationBreakdownTooltip active={props.active} payload={props.payload} />
                  )}
                />
                <Bar dataKey="count" isAnimationActive={false} maxBarSize={28} minPointSize={4}>
                  {data.map((formation) => (
                    <Cell key={formation.key} fill={formation.color} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </div>
          <ul aria-label={t("Formations")} className="sr-only">
            {data.map((formation) => (
              <li key={formation.key}>
                {formation.label}: {numberFormatter.format(formation.count)} (
                {formatPercent(formation.percentage)})
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

function buildFormationCounts(formations: CombatLabFormation[]): {
  entries: FormationCount[];
  total: number;
} {
  const countsByFormationId = new Map<number, number>();

  for (const formation of formations) {
    if (formation.count <= 0) {
      continue;
    }

    const formationId = WEDGE_FORMATION_IDS.has(formation.id) ? WEDGE_FORMATION_ID : formation.id;
    countsByFormationId.set(
      formationId,
      (countsByFormationId.get(formationId) ?? 0) + formation.count
    );
  }

  const sortedEntries = [...countsByFormationId]
    .map<FormationCount>(([formationId, count]) => ({
      key: formationId.toString(),
      formationId,
      count,
    }))
    .sort((left, right) => right.count - left.count);
  const total = sortedEntries.reduce((sum, formation) => sum + formation.count, 0);

  if (sortedEntries.length <= MAX_FORMATION_BARS) {
    return { entries: sortedEntries, total };
  }

  const entries = sortedEntries.slice(0, VISIBLE_FORMATION_BARS_WITH_OTHER);
  const otherCount = sortedEntries
    .slice(VISIBLE_FORMATION_BARS_WITH_OTHER)
    .reduce((sum, formation) => sum + formation.count, 0);

  entries.push({
    key: "other",
    formationId: null,
    count: otherCount,
  });

  return { entries, total };
}

function FormationBreakdownTooltip({ active, payload }: { active?: boolean; payload?: unknown }) {
  const t = useExtracted();

  if (!active || !Array.isArray(payload) || payload.length === 0) {
    return null;
  }

  const entry = payload[0];
  if (!entry || typeof entry !== "object" || !("payload" in entry)) {
    return null;
  }

  const formation = entry.payload as Partial<FormationDatum>;
  if (
    typeof formation.label !== "string" ||
    typeof formation.count !== "number" ||
    typeof formation.percentage !== "number"
  ) {
    return null;
  }

  return (
    <div className="min-w-40 rounded-lg border border-zinc-950/10 bg-white px-3 py-2 text-sm shadow-lg dark:border-white/10 dark:bg-zinc-900">
      <div className="flex items-center gap-2">
        <span
          aria-hidden="true"
          className="size-2.5 shrink-0 rounded-full"
          style={{ backgroundColor: formation.color }}
        />
        <span className="font-medium text-zinc-700 dark:text-zinc-100">{formation.label}</span>
      </div>
      <dl className="mt-2 grid grid-cols-[1fr_auto] gap-x-4 gap-y-1 text-zinc-600 dark:text-zinc-300">
        <dt>{t("Count")}</dt>
        <dd className="text-right tabular-nums">{numberFormatter.format(formation.count)}</dd>
        <dt>{t("Usage")}</dt>
        <dd className="text-right tabular-nums">{formatPercent(formation.percentage)}</dd>
      </dl>
    </div>
  );
}

function FormationAxisTick({
  payload,
  x = 0,
  y = 0,
}: {
  payload?: { value?: string | number };
  x?: number;
  y?: number;
}) {
  const label = String(payload?.value ?? "");

  return (
    <foreignObject height={20} width={Math.max(0, x - 8)} x={0} y={y - 10}>
      <div className="truncate text-right text-[11px] leading-5 text-zinc-500" title={label}>
        {label}
      </div>
    </foreignObject>
  );
}
