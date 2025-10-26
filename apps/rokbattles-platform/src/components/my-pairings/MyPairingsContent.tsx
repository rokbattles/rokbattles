"use client";

import { useContext, useEffect, useMemo, useState } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";
import { PairingMetricCard } from "@/components/my-pairings/PairingMetricCard";
import { Select } from "@/components/ui/Select";
import { Text } from "@/components/ui/Text";
import { getCommanderName } from "@/hooks/useCommanderName";
import { useCurrentUser } from "@/hooks/useCurrentUser";
import { type UseMyPairingsResult, useMyPairings } from "@/hooks/useMyPairings";
import { formatDurationShort } from "@/lib/datetime";

const numberFormatter = new Intl.NumberFormat("en-US");
const perSecondFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 1,
  minimumFractionDigits: 0,
});

type PairingMetricDefinition = {
  key: string;
  label: string;
  value: number;
  previousValue: number;
  trendDirection: "increase" | "decrease";
  formatValue?: (value: number) => string;
};

function formatNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "0";
  }

  return numberFormatter.format(Math.round(value));
}

function formatDurationSeconds(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return "0s";
  }

  const base = 1;
  return formatDurationShort(base, base + value);
}

function formatPerSecond(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return "0/s";
  }

  return `${perSecondFormatter.format(value)}/s`;
}

function formatPeriod(period: UseMyPairingsResult["period"]) {
  if (!period) {
    return null;
  }

  const start = new Date(period.start);
  const end = new Date(period.end);

  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) {
    return null;
  }

  const formatter = new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
  });

  return `${formatter.format(start)} - ${formatter.format(end)} UTC`;
}

function createPairingKey(primaryId: number, secondaryId: number) {
  return `${primaryId}:${secondaryId}`;
}

function formatCommanderPair(primaryId: number, secondaryId: number) {
  const primaryName = getCommanderName(primaryId) ?? primaryId;
  const secondaryName = getCommanderName(secondaryId) ?? secondaryId;

  if (secondaryId <= 0 || !secondaryName) {
    return primaryName;
  }

  return `${primaryName} / ${secondaryName}`;
}

export function MyPairingsContent() {
  const { user, loading } = useCurrentUser();
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("My Pairings page must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;
  const { data, loading: pairingsLoading, error, period } = useMyPairings();
  const [selectedKey, setSelectedKey] = useState<string | null>(null);

  const periodLabel = useMemo(() => formatPeriod(period), [period]);

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
            createPairingKey(
              pairing.primaryCommanderId,
              pairing.secondaryCommanderId,
            ) === current,
        )
      ) {
        return current;
      }

      const first = data[0];
      return createPairingKey(
        first.primaryCommanderId,
        first.secondaryCommanderId,
      );
    });
  }, [data]);

  const pairingOptions = useMemo(
    () =>
      data.map((pairing) => ({
        value: createPairingKey(
          pairing.primaryCommanderId,
          pairing.secondaryCommanderId,
        ),
        label: formatCommanderPair(
          pairing.primaryCommanderId,
          pairing.secondaryCommanderId,
        ),
      })),
    [data],
  );

  const selectedPairing = useMemo(
    () =>
      data.find(
        (pairing) =>
          createPairingKey(
            pairing.primaryCommanderId,
            pairing.secondaryCommanderId,
          ) === selectedKey,
      ),
    [data, selectedKey],
  );

  const metrics = useMemo<PairingMetricDefinition[]>(() => {
    if (!selectedPairing) {
      return [];
    }

    const { totals, previousTotals } = selectedPairing;
    const totalDurationSeconds =
      totals.battleDuration > 0 ? totals.battleDuration / 1000 : 0;
    const previousDurationSeconds =
      previousTotals.battleDuration > 0
        ? previousTotals.battleDuration / 1000
        : 0;
    const averageBattleDurationSeconds =
      selectedPairing.count > 0
        ? totalDurationSeconds / selectedPairing.count
        : 0;
    const previousAverageBattleDurationSeconds =
      selectedPairing.previousCount > 0
        ? previousDurationSeconds / selectedPairing.previousCount
        : 0;
    const dpsPerSecond =
      totalDurationSeconds > 0 ? totals.dps / totalDurationSeconds : 0;
    const previousDpsPerSecond =
      previousDurationSeconds > 0
        ? previousTotals.dps / previousDurationSeconds
        : 0;
    const spsPerSecond =
      totalDurationSeconds > 0 ? totals.sps / totalDurationSeconds : 0;
    const previousSpsPerSecond =
      previousDurationSeconds > 0
        ? previousTotals.sps / previousDurationSeconds
        : 0;
    const tpsPerSecond =
      totalDurationSeconds > 0 ? totals.tps / totalDurationSeconds : 0;
    const previousTpsPerSecond =
      previousDurationSeconds > 0
        ? previousTotals.tps / previousDurationSeconds
        : 0;

    return [
      {
        key: "killScore",
        label: "Kill Points",
        value: totals.killScore,
        previousValue: previousTotals.killScore,
        trendDirection: "increase" as const,
      },
      {
        key: "deaths",
        label: "Deaths",
        value: totals.deaths,
        previousValue: previousTotals.deaths,
        trendDirection: "decrease" as const,
      },
      {
        key: "severelyWounded",
        label: "Severely Wounded",
        value: totals.severelyWounded,
        previousValue: previousTotals.severelyWounded,
        trendDirection: "decrease" as const,
      },
      {
        key: "wounded",
        label: "Slightly Wounded",
        value: totals.wounded,
        previousValue: previousTotals.wounded,
        trendDirection: "decrease" as const,
      },
      {
        key: "averageBattleDuration",
        label: "Avg. Battle Duration",
        value: averageBattleDurationSeconds,
        previousValue: previousAverageBattleDurationSeconds,
        trendDirection: "decrease" as const,
        formatValue: formatDurationSeconds,
      },
      {
        key: "dps",
        label: "DPS (Damage)",
        value: dpsPerSecond,
        previousValue: previousDpsPerSecond,
        trendDirection: "increase" as const,
        formatValue: formatPerSecond,
      },
      {
        key: "sps",
        label: "SPS (Sevs. Given)",
        value: spsPerSecond,
        previousValue: previousSpsPerSecond,
        trendDirection: "increase" as const,
        formatValue: formatPerSecond,
      },
      {
        key: "tps",
        label: "TPS (Sevs. Taken)",
        value: tpsPerSecond,
        previousValue: previousTpsPerSecond,
        trendDirection: "decrease" as const,
        formatValue: formatPerSecond,
      },
      {
        key: "enemyKillScore",
        label: "Enemy Kill Points",
        value: totals.enemyKillScore,
        previousValue: previousTotals.enemyKillScore,
        trendDirection: "increase" as const,
      },
      {
        key: "enemyDeaths",
        label: "Enemy Deaths",
        value: totals.enemyDeaths,
        previousValue: previousTotals.enemyDeaths,
        trendDirection: "increase" as const,
      },
      {
        key: "enemySeverelyWounded",
        label: "Enemy Severely Wounded",
        value: totals.enemySeverelyWounded,
        previousValue: previousTotals.enemySeverelyWounded,
        trendDirection: "increase" as const,
      },
      {
        key: "enemyWounded",
        label: "Enemy Slightly Wounded",
        value: totals.enemyWounded,
        previousValue: previousTotals.enemyWounded,
        trendDirection: "increase" as const,
      },
    ];
  }, [selectedPairing]);

  if (loading) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400">
        Loading your account&hellip;
      </p>
    );
  }

  if (!user) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400">
        You must be logged in to view this page.
      </p>
    );
  }

  if (!activeGovernor) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400">
        You must have a claimed governor to view this page.
      </p>
    );
  }

  return (
    <>
      <div className="mt-8 flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="space-y-4">
          <Select
            name="pairing"
            value={selectedKey ?? ""}
            onChange={(event) => setSelectedKey(event.target.value || null)}
            disabled={pairingsLoading || pairingOptions.length === 0}
          >
            <option value="" disabled hidden>
              Select a pairing
            </option>
            {pairingOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </Select>
          <Text>
            {selectedPairing && periodLabel
              ? `Aggregated from ${formatNumber(selectedPairing.count)} battles in ${periodLabel}.`
              : selectedPairing
                ? `Aggregated from ${formatNumber(selectedPairing.count)} battles.`
                : pairingsLoading
                  ? "Loading pairings..."
                  : "Select a pairing to see results."}
          </Text>
        </div>
      </div>
      {error ? (
        <Text className="mt-6 text-sm/6 text-red-600 dark:text-red-400">
          Failed to load pairings: {error}
        </Text>
      ) : null}
      {!pairingsLoading && !error && pairingOptions.length === 0 ? (
        <Text className="mt-6 text-sm/6 text-zinc-500 dark:text-zinc-400">
          No pairings found for this period.
        </Text>
      ) : null}
      {selectedPairing ? (
        <div className="mt-6 grid gap-8 sm:grid-cols-2 xl:grid-cols-4">
          {metrics.map((metric) => (
            <PairingMetricCard
              key={metric.key}
              label={metric.label}
              value={metric.value}
              previousValue={metric.previousValue}
              trendDirection={metric.trendDirection}
              formatValue={metric.formatValue ?? formatNumber}
            />
          ))}
        </div>
      ) : null}
    </>
  );
}
