"use client";

import { useContext, useEffect, useState } from "react";
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
import { GovernorContext } from "@/components/context/GovernorContext";
import { PairingMetricCard } from "@/components/my-pairings/PairingMetricCard";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/Listbox";
import { Text } from "@/components/ui/Text";
import { getCommanderName } from "@/hooks/useCommanderName";
import { useCurrentUser } from "@/hooks/useCurrentUser";
import { type GovernorMarchTotals, usePairings } from "@/hooks/usePairings";
import { formatDurationShort } from "@/lib/datetime";

const numberFormatter = new Intl.NumberFormat("en-US");
const perSecondFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 1,
  minimumFractionDigits: 0,
});
const monthFormatter = new Intl.DateTimeFormat("en-US", { month: "short", timeZone: "UTC" });
const monthWithYearFormatter = new Intl.DateTimeFormat("en-US", {
  month: "long",
  year: "numeric",
  timeZone: "UTC",
});

type PairingMetricDefinition = {
  key: string;
  label: string;
  value: number;
  previousValue?: number;
  trendDirection: "increase" | "decrease";
  formatValue?: (value: number) => string;
  description?: string;
  comparisonLabel?: string;
};

type MonthPoint = {
  key: string;
  label: string;
  battles: number;
  isSelected: boolean;
};

function createEmptyTotals(): GovernorMarchTotals {
  return {
    killScore: 0,
    deaths: 0,
    severelyWounded: 0,
    wounded: 0,
    enemyKillScore: 0,
    enemyDeaths: 0,
    enemySeverelyWounded: 0,
    enemyWounded: 0,
    dps: 0,
    sps: 0,
    tps: 0,
    battleDuration: 0,
  };
}

function formatNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "0";
  }

  return numberFormatter.format(Math.round(value));
}

function formatDurationSeconds(value: number): string {
  if (!Number.isFinite(value)) {
    return "0s";
  }

  const base = 1;
  return formatDurationShort(base, base + value);
}

function formatPerSecond(value: number): string {
  if (!Number.isFinite(value)) {
    return "0/s";
  }

  return `${perSecondFormatter.format(value)}/s`;
}

function createPairingKey(primaryId: number, secondaryId: number) {
  return `${primaryId}:${secondaryId}`;
}

function formatCommanderPair(primaryId: number, secondaryId: number) {
  const primaryName = getCommanderName(primaryId) ?? primaryId;
  const secondaryName = getCommanderName(secondaryId) ?? secondaryId;

  if (!secondaryName) {
    return primaryName;
  }

  return `${primaryName} / ${secondaryName}`;
}

function createMonthKey(date: Date) {
  return `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, "0")}`;
}

function parseMonthKey(key: string | null) {
  if (!key) {
    return null;
  }

  const [yearStr, monthStr] = key.split("-");
  const year = Number(yearStr);
  const month = Number(monthStr) - 1;
  if (!Number.isFinite(year) || !Number.isFinite(month)) {
    return null;
  }

  return new Date(Date.UTC(year, month, 1));
}

function buildMonthsForYear(year: number) {
  return Array.from({ length: 12 }, (_, monthIndex) => {
    const date = new Date(Date.UTC(year, monthIndex, 1));
    return {
      key: createMonthKey(date),
      label: monthFormatter.format(date),
    };
  });
}

export function MyPairingsContent() {
  const { user, loading } = useCurrentUser();
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("My Pairings page must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;
  const { data, loading: pairingsLoading, error, year } = usePairings();
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [selectedMonthKey, setSelectedMonthKey] = useState<string | null>(null);

  const chartYear = year ?? 2025;

  useEffect(() => {
    if (data.length === 0) {
      setSelectedKey(null);
      return;
    }

    setSelectedKey((current) => {
      if (
        current &&
        data.some(
          (pairing) =>
            createPairingKey(pairing.primaryCommanderId, pairing.secondaryCommanderId) === current
        )
      ) {
        return current;
      }

      const first = data[0];
      return createPairingKey(first.primaryCommanderId, first.secondaryCommanderId);
    });
  }, [data]);

  const pairingOptions = data.map((pairing) => ({
    value: createPairingKey(pairing.primaryCommanderId, pairing.secondaryCommanderId),
    label: formatCommanderPair(pairing.primaryCommanderId, pairing.secondaryCommanderId),
  }));

  const selectedPairing = data.find(
    (pairing) =>
      createPairingKey(pairing.primaryCommanderId, pairing.secondaryCommanderId) === selectedKey
  );

  const months = buildMonthsForYear(chartYear);
  const monthlyByKey = new Map<string, { count: number; totals: GovernorMarchTotals }>();
  if (selectedPairing) {
    for (const month of selectedPairing.monthly ?? []) {
      monthlyByKey.set(month.monthKey, { count: month.count, totals: month.totals });
    }
  }

  useEffect(() => {
    const monthsForYear = buildMonthsForYear(chartYear);
    if (!monthsForYear.length) {
      return;
    }

    const monthlyMap = new Map<string, { count: number; totals: GovernorMarchTotals }>();
    if (selectedPairing) {
      for (const month of selectedPairing.monthly ?? []) {
        monthlyMap.set(month.monthKey, { count: month.count, totals: month.totals });
      }
    }

    const now = new Date();
    const currentMonthIndex =
      now.getUTCFullYear() === chartYear ? now.getUTCMonth() : Math.min(now.getUTCMonth(), 11);
    const desiredMonthKey = monthsForYear[currentMonthIndex]?.key ?? monthsForYear[0]?.key ?? null;
    const latestWithData =
      monthsForYear
        .slice(0, Math.min(currentMonthIndex, monthsForYear.length - 1) + 1)
        .reverse()
        .find((month) => (monthlyMap.get(month.key)?.count ?? 0) > 0)?.key ?? null;

    setSelectedMonthKey((current) => {
      if (current && monthsForYear.some((month) => month.key === current)) {
        return current;
      }

      return latestWithData ?? desiredMonthKey ?? null;
    });
  }, [chartYear, selectedPairing]);

  const chartData: MonthPoint[] = months.map((month) => {
    const entry = monthlyByKey.get(month.key);
    return {
      ...month,
      battles: entry?.count ?? 0,
      isSelected: month.key === selectedMonthKey,
    };
  });

  const parsedSelectedMonth = parseMonthKey(selectedMonthKey);
  const selectedMonthLabel = parsedSelectedMonth
    ? monthWithYearFormatter.format(parsedSelectedMonth)
    : null;

  const monthStats = (() => {
    const defaults = {
      totals: createEmptyTotals(),
      count: 0,
      comparisonTotals: undefined as GovernorMarchTotals | undefined,
      comparisonCount: undefined as number | undefined,
      comparisonLabel: "No prior month to compare",
    };

    if (!selectedPairing || !selectedMonthKey) {
      return defaults;
    }

    const monthIndex = months.findIndex((month) => month.key === selectedMonthKey);
    const previousMonthKey = monthIndex > 0 ? months[monthIndex - 1]?.key : null;
    const selectedTotals = monthlyByKey.get(selectedMonthKey);
    const comparisonTotals = previousMonthKey ? monthlyByKey.get(previousMonthKey) : undefined;

    return {
      totals: selectedTotals?.totals ?? createEmptyTotals(),
      count: selectedTotals?.count ?? 0,
      comparisonTotals: comparisonTotals?.totals,
      comparisonCount: comparisonTotals?.count,
      comparisonLabel: previousMonthKey
        ? `vs ${monthWithYearFormatter.format(parseMonthKey(previousMonthKey) ?? new Date())}`
        : "No earlier month to compare",
    };
  })();

  const totalDurationSeconds =
    monthStats.totals.battleDuration > 0 ? monthStats.totals.battleDuration / 1000 : 0;
  const comparisonDurationSeconds =
    monthStats.comparisonTotals && monthStats.comparisonTotals.battleDuration > 0
      ? monthStats.comparisonTotals.battleDuration / 1000
      : 0;

  const averageBattleDurationSeconds =
    monthStats.count > 0 ? totalDurationSeconds / monthStats.count : 0;
  const comparisonAverageBattleDurationSeconds =
    monthStats.comparisonTotals && monthStats.comparisonCount && monthStats.comparisonCount > 0
      ? comparisonDurationSeconds / monthStats.comparisonCount
      : undefined;

  const dpsPerSecond = totalDurationSeconds > 0 ? monthStats.totals.dps / totalDurationSeconds : 0;
  const previousDpsPerSecond =
    monthStats.comparisonTotals && comparisonDurationSeconds > 0
      ? monthStats.comparisonTotals.dps / comparisonDurationSeconds
      : undefined;
  const spsPerSecond = totalDurationSeconds > 0 ? monthStats.totals.sps / totalDurationSeconds : 0;
  const previousSpsPerSecond =
    monthStats.comparisonTotals && comparisonDurationSeconds > 0
      ? monthStats.comparisonTotals.sps / comparisonDurationSeconds
      : undefined;
  const tpsPerSecond = totalDurationSeconds > 0 ? monthStats.totals.tps / totalDurationSeconds : 0;
  const previousTpsPerSecond =
    monthStats.comparisonTotals && comparisonDurationSeconds > 0
      ? monthStats.comparisonTotals.tps / comparisonDurationSeconds
      : undefined;

  const metrics: PairingMetricDefinition[] = selectedPairing
    ? [
        {
          key: "battleCount",
          label: "Battles",
          value: monthStats.count,
          previousValue: monthStats.comparisonCount,
          trendDirection: "increase",
          formatValue: formatNumber,
          description:
            "How many battle reports you generated with this pairing in the selected month.",
          comparisonLabel: monthStats.comparisonLabel,
        },
        {
          key: "killScore",
          label: "Kill Points",
          value: monthStats.totals.killScore,
          previousValue: monthStats.comparisonTotals?.killScore,
          trendDirection: "increase",
          formatValue: formatNumber,
          description: "Total kill points you earned while using this pairing.",
          comparisonLabel: monthStats.comparisonLabel,
        },
        {
          key: "enemyKillScore",
          label: "Enemy Kill Points",
          value: monthStats.totals.enemyKillScore,
          previousValue: monthStats.comparisonTotals?.enemyKillScore,
          trendDirection: "increase",
          formatValue: formatNumber,
          description: "Total kill points your opponents earned against you.",
          comparisonLabel: monthStats.comparisonLabel,
        },
        {
          key: "severelyWounded",
          label: "Severely Wounded (Taken)",
          value: monthStats.totals.severelyWounded,
          previousValue: monthStats.comparisonTotals?.severelyWounded,
          trendDirection: "decrease",
          formatValue: formatNumber,
          description:
            "Number of your troops that became severely wounded while using this pairing.",
          comparisonLabel: monthStats.comparisonLabel,
        },
        {
          key: "enemySeverelyWounded",
          label: "Severely Wounded (Inflicted)",
          value: monthStats.totals.enemySeverelyWounded,
          previousValue: monthStats.comparisonTotals?.enemySeverelyWounded,
          trendDirection: "increase",
          formatValue: formatNumber,
          description: "Number of enemy troops you caused to become severely wounded.",
          comparisonLabel: monthStats.comparisonLabel,
        },
        {
          key: "averageBattleDuration",
          label: "Avg. Battle Duration",
          value: averageBattleDurationSeconds,
          previousValue: comparisonAverageBattleDurationSeconds,
          trendDirection: "decrease",
          formatValue: formatDurationSeconds,
          description: "Average length of battles recorded for this pairing in the selected month.",
          comparisonLabel: monthStats.comparisonLabel,
        },
        {
          key: "dps",
          label: "Damage Per Second (DPS)",
          value: dpsPerSecond,
          previousValue: previousDpsPerSecond,
          trendDirection: "increase",
          formatValue: formatPerSecond,
          description: "Average amount of damage you inflict per second with this pairing.",
          comparisonLabel: monthStats.comparisonLabel,
        },
        {
          key: "sps",
          label: "Sevs Per Second (SPS)",
          value: spsPerSecond,
          previousValue: previousSpsPerSecond,
          trendDirection: "increase",
          formatValue: formatPerSecond,
          description: "Rate at which you inflict severely wounded troops each second.",
          comparisonLabel: monthStats.comparisonLabel,
        },
        {
          key: "tps",
          label: "Sevs Taken Per Second (TPS)",
          value: tpsPerSecond,
          previousValue: previousTpsPerSecond,
          trendDirection: "decrease",
          formatValue: formatPerSecond,
          description: "Rate at which your troops become severely wounded each second.",
          comparisonLabel: monthStats.comparisonLabel,
        },
      ]
    : [];

  if (loading) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        Loading your account&hellip;
      </p>
    );
  }

  if (!user) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        You must be logged in to view this page.
      </p>
    );
  }

  if (!activeGovernor) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        You must have a claimed governor to view this page.
      </p>
    );
  }

  return (
    <div className="mt-6 space-y-6">
      <div className="flex flex-col gap-4 border-b border-zinc-200/80 pb-4 dark:border-white/10 lg:flex-row lg:items-center lg:justify-between">
        <Text className="text-sm text-zinc-950 dark:text-white">
          Choose a pairing to see how it performed, filterable by month.
        </Text>
        <div className="w-full max-w-md">
          <Listbox<string | null>
            name="pairing"
            value={selectedKey ?? null}
            placeholder="Select a pairing"
            onChange={(value) => setSelectedKey(value)}
            disabled={pairingsLoading || pairingOptions.length === 0}
          >
            {pairingOptions.map((option) => (
              <ListboxOption key={option.value} value={option.value}>
                <ListboxLabel>{option.label}</ListboxLabel>
              </ListboxOption>
            ))}
          </Listbox>
        </div>
      </div>

      {error ? (
        <Text className="text-sm/6 text-red-600 dark:text-red-400" role="status" aria-live="polite">
          Failed to load pairings: {error}
        </Text>
      ) : null}
      {!pairingsLoading && !error && pairingOptions.length === 0 ? (
        <Text
          className="text-sm/6 text-zinc-500 dark:text-zinc-400"
          role="status"
          aria-live="polite"
        >
          No pairings found for this period.
        </Text>
      ) : null}

      {selectedPairing ? (
        <>
          <div className="border border-zinc-200/60 p-4 dark:border-white/10">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <div>
                <div className="text-base font-semibold text-zinc-950 dark:text-white">
                  Monthly battles in {chartYear}
                </div>
                <Text className="text-sm text-zinc-600 dark:text-zinc-400">
                  Click a bar to switch months, alternatively use the select menu.
                </Text>
              </div>
              <div className="w-full max-w-[220px] sm:w-auto">
                <Listbox<string | null>
                  value={selectedMonthKey ?? null}
                  onChange={(value) => setSelectedMonthKey(value)}
                  name="month"
                  placeholder="Select month"
                >
                  {months.map((month) => (
                    <ListboxOption key={month.key} value={month.key}>
                      <ListboxLabel>
                        {monthWithYearFormatter.format(parseMonthKey(month.key) ?? new Date())}
                      </ListboxLabel>
                    </ListboxOption>
                  ))}
                </Listbox>
              </div>
            </div>
            <div className="mt-4 h-64">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#d4d4d8" opacity={0.15} />
                  <XAxis
                    dataKey="label"
                    stroke="#a1a1aa"
                    tickLine={false}
                    axisLine={false}
                    tick={{ fontSize: 12 }}
                  />
                  <YAxis
                    stroke="#a1a1aa"
                    tickLine={false}
                    axisLine={false}
                    tick={{ fontSize: 12 }}
                    allowDecimals={false}
                  />
                  <RechartsTooltip
                    cursor={{ fill: "rgba(24,24,27,0.05)" }}
                    contentStyle={{
                      backgroundColor: "rgba(24,24,27,0.9)",
                      border: "1px solid rgba(228,228,231,0.25)",
                      borderRadius: "12px",
                      color: "#fafafa",
                    }}
                    labelStyle={{ color: "#fafafa" }}
                    itemStyle={{ color: "#fafafa" }}
                    formatter={(value: number) => [formatNumber(value), "Battles"]}
                  />
                  <Bar dataKey="battles" radius={[8, 8, 2, 2]}>
                    {chartData.map((entry) => (
                      <Cell
                        key={entry.key}
                        fill={entry.isSelected ? "#22c55e" : "#a1a1aa"}
                        className="cursor-pointer transition-all duration-300"
                        onClick={() => setSelectedMonthKey(entry.key)}
                      />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>

          <div className="flex flex-col gap-2 border-b border-zinc-200/60 pb-4 dark:border-white/10 sm:flex-row sm:items-center sm:justify-between">
            <div className="text-lg font-semibold text-zinc-950 dark:text-white">
              {selectedMonthLabel ?? "Select a month"}
            </div>
          </div>

          <div className="grid gap-6 sm:grid-cols-2 xl:grid-cols-3">
            {metrics.map((metric) => (
              <PairingMetricCard
                key={metric.key}
                label={metric.label}
                value={metric.value}
                previousValue={metric.previousValue}
                trendDirection={metric.trendDirection}
                formatValue={metric.formatValue ?? formatNumber}
                description={metric.description}
                comparisonLabel={metric.comparisonLabel}
              />
            ))}
          </div>
        </>
      ) : null}
    </div>
  );
}
