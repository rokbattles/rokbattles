"use client";

import { useExtracted } from "next-intl";
import { use, useEffect, useId, useMemo, useState } from "react";
import { PairingsFilters } from "@/components/my-pairings/pairings-filters";
import { PairingsLoadoutBreakdown } from "@/components/my-pairings/pairings-loadout-breakdown";
import { type LoadoutCard, PairingsLoadouts } from "@/components/my-pairings/pairings-loadouts";
import { Text } from "@/components/ui/text";
import { getCommanderName } from "@/hooks/use-commander-name";
import { usePairingLoadouts } from "@/hooks/use-pairing-loadouts";
import { usePairingOpponents } from "@/hooks/use-pairing-opponents";
import { usePairings } from "@/hooks/use-pairings";
import { formatDurationShort } from "@/lib/datetime";
import type {
  LoadoutGranularity,
  LoadoutSnapshot,
  OpponentGranularity,
  PairingsActivity,
  PairingsBattleType,
} from "@/lib/pairings";
import { formatPerSecond } from "@/lib/statistics-format";
import { GovernorContext } from "@/providers/governor-context";

const numberFormatter = new Intl.NumberFormat("en-US");
const percentFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 1,
  minimumFractionDigits: 0,
});

const ALL_LOADOUT_KEY = "all-loadouts";
const EMPTY_LOADOUT: LoadoutSnapshot = {
  equipment: [],
  armaments: [],
  inscriptions: [],
  formation: null,
};

function formatNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "0";
  }

  return numberFormatter.format(Math.round(value));
}

function formatPercent(value: number): string {
  if (!Number.isFinite(value)) {
    return "0%";
  }

  return `${percentFormatter.format(value)}%`;
}

function formatDurationSeconds(valueSeconds: number) {
  if (!Number.isFinite(valueSeconds) || valueSeconds <= 0) {
    return "0s";
  }

  const base = 1;
  return formatDurationShort(base, base + valueSeconds);
}

function ratePerSecond(value: number, durationMillis: number) {
  if (!Number.isFinite(value) || !Number.isFinite(durationMillis) || durationMillis <= 0) {
    return 0;
  }

  return value / (durationMillis / 1000);
}

function createPairingKey(primaryId: number, secondaryId: number) {
  return `${primaryId}:${secondaryId}`;
}

function formatCommanderPair(primaryId: number, secondaryId: number, unknownLabel: string) {
  const primaryName = primaryId > 0 ? (getCommanderName(primaryId) ?? primaryId) : unknownLabel;
  const secondaryName = secondaryId > 0 ? (getCommanderName(secondaryId) ?? secondaryId) : null;

  if (!secondaryName) {
    return String(primaryName);
  }

  return `${primaryName} / ${secondaryName}`;
}

export function MyPairingsContent() {
  const governorContext = use(GovernorContext);

  if (!governorContext) {
    throw new Error("My Pairings must be used within a GovernorProvider");
  }

  const t = useExtracted();
  const { activeGovernor } = governorContext;

  const [startDate, setStartDate] = useState<string>("");
  const [endDate, setEndDate] = useState<string>("");
  const [excludedActivities, setExcludedActivities] = useState<PairingsActivity[]>([]);
  const [excludedBattles, setExcludedBattles] = useState<PairingsBattleType[]>([]);
  const hasCustomRange = Boolean(startDate && endDate);
  const rangeStartDate = hasCustomRange ? startDate : undefined;
  const rangeEndDate = hasCustomRange ? endDate : undefined;
  const {
    data,
    loading: pairingsLoading,
    error: pairingsError,
  } = usePairings({
    governorId: activeGovernor?.governorId,
    startDate: rangeStartDate,
    endDate: rangeEndDate,
    excludeActivities: excludedActivities,
    excludeBattles: excludedBattles,
  });
  const [selectedPairingKey, setSelectedPairingKey] = useState<string | null>(null);
  const [loadoutGranularity, setLoadoutGranularity] = useState<LoadoutGranularity>("simplified");
  const [selectedLoadoutKey, setSelectedLoadoutKey] = useState<string | null>(ALL_LOADOUT_KEY);
  const [loadoutsFetchStarted, setLoadoutsFetchStarted] = useState(false);
  const [loadoutsReady, setLoadoutsReady] = useState(false);
  const [showAllOpponents, setShowAllOpponents] = useState(false);
  const opponentsId = useId();

  useEffect(() => {
    if (data.length === 0) {
      setSelectedPairingKey(null);
      return;
    }

    setSelectedPairingKey((current) => {
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

  const pairingOptions = useMemo(
    () =>
      data.map((pairing) => ({
        value: createPairingKey(pairing.primaryCommanderId, pairing.secondaryCommanderId),
        label: formatCommanderPair(
          pairing.primaryCommanderId,
          pairing.secondaryCommanderId,
          t("Unknown commander")
        ),
      })),
    [data, t]
  );

  const selectedPairing = data.find(
    (pairing) =>
      createPairingKey(pairing.primaryCommanderId, pairing.secondaryCommanderId) ===
      selectedPairingKey
  );
  const hasSelectedPairing = Boolean(selectedPairing);

  const canLoadLoadouts = hasSelectedPairing && !pairingsLoading && !pairingsError;
  const {
    data: loadouts,
    loading: loadoutsLoading,
    error: loadoutsError,
  } = usePairingLoadouts({
    governorId: activeGovernor?.governorId,
    primaryCommanderId: canLoadLoadouts ? (selectedPairing?.primaryCommanderId ?? null) : null,
    secondaryCommanderId: canLoadLoadouts ? (selectedPairing?.secondaryCommanderId ?? null) : null,
    granularity: loadoutGranularity,
    startDate: rangeStartDate,
    endDate: rangeEndDate,
    excludeActivities: excludedActivities,
    excludeBattles: excludedBattles,
  });
  const loadoutsResetKey = useMemo(
    () =>
      [
        activeGovernor?.governorId ?? "none",
        selectedPairingKey ?? "none",
        loadoutGranularity,
        rangeStartDate ?? "none",
        rangeEndDate ?? "none",
        excludedActivities.join(",") || "none",
        excludedBattles.join(",") || "none",
        canLoadLoadouts ? "ready" : "idle",
      ].join("|"),
    [
      activeGovernor?.governorId,
      selectedPairingKey,
      loadoutGranularity,
      rangeStartDate,
      rangeEndDate,
      excludedActivities,
      excludedBattles,
      canLoadLoadouts,
    ]
  );
  const opponentsResetKey = useMemo(
    () =>
      [
        selectedPairingKey ?? "none",
        selectedLoadoutKey ?? "none",
        loadoutGranularity,
        rangeStartDate ?? "none",
        rangeEndDate ?? "none",
        excludedActivities.join(",") || "none",
        excludedBattles.join(",") || "none",
      ].join("|"),
    [
      selectedPairingKey,
      selectedLoadoutKey,
      loadoutGranularity,
      rangeStartDate,
      rangeEndDate,
      excludedActivities,
      excludedBattles,
    ]
  );

  useEffect(() => {
    void loadoutsResetKey;
    setLoadoutsFetchStarted(false);
    setLoadoutsReady(false);
  }, [loadoutsResetKey]);

  useEffect(() => {
    if (!canLoadLoadouts) {
      setLoadoutsFetchStarted(false);
      setLoadoutsReady(false);
      return;
    }

    if (loadoutsLoading) {
      setLoadoutsFetchStarted(true);
      return;
    }

    if (loadoutsFetchStarted) {
      setLoadoutsReady(true);
    }
  }, [canLoadLoadouts, loadoutsLoading, loadoutsFetchStarted]);

  const loadoutCards = useMemo<LoadoutCard[]>(() => {
    if (!selectedPairing) {
      return [];
    }

    const allLoadouts: LoadoutCard = {
      key: ALL_LOADOUT_KEY,
      label: t("All loadouts"),
      count: selectedPairing.count,
      totals: selectedPairing.totals,
      loadout: EMPTY_LOADOUT,
    };

    const cards = loadouts.map<LoadoutCard>((loadout, index) => ({
      ...loadout,
      label: t("Loadout {index}", { index: (index + 1).toString() }),
    }));

    return [allLoadouts, ...cards];
  }, [loadouts, selectedPairing, t]);

  useEffect(() => {
    if (!selectedPairing) {
      setSelectedLoadoutKey(ALL_LOADOUT_KEY);
      return;
    }

    const keys = new Set(loadoutCards.map((card) => card.key));
    setSelectedLoadoutKey((current) => {
      if (current && keys.has(current)) {
        return current;
      }

      return ALL_LOADOUT_KEY;
    });
  }, [loadoutCards, selectedPairing]);

  const selectedLoadoutCard =
    loadoutCards.find((loadout) => loadout.key === selectedLoadoutKey) ?? null;
  const hasSelectedLoadout = Boolean(selectedLoadoutCard);
  const generalStats = useMemo(() => {
    if (!selectedLoadoutCard) {
      return [];
    }

    const durationSeconds = selectedLoadoutCard.totals.battleDuration / 1000;
    const avgDurationSeconds =
      selectedLoadoutCard.count > 0 ? durationSeconds / selectedLoadoutCard.count : 0;

    return [
      {
        id: "battles",
        name: t("Battles"),
        value: formatNumber(selectedLoadoutCard.count),
        description: t("Total battle reports recorded for this loadout."),
      },
      {
        id: "killPoints",
        name: t("Kill Points"),
        value: formatNumber(selectedLoadoutCard.totals.killScore),
        description: t("Total kill points earned while using this pairing."),
      },
      {
        id: "enemyKillPoints",
        name: t("Opponent Kill Points"),
        value: formatNumber(selectedLoadoutCard.totals.enemyKillScore),
        description: t("Total kill points earned by opponents against this pairing."),
      },
      {
        id: "severelyWounded",
        name: t("Severely Wounded (Taken)"),
        value: formatNumber(selectedLoadoutCard.totals.severelyWounded),
        description: t("Number of troops that became severely wounded while using this pairing."),
      },
      {
        id: "enemySeverelyWounded",
        name: t("Severely Wounded (Inflicted)"),
        value: formatNumber(selectedLoadoutCard.totals.enemySeverelyWounded),
        description: t("Number of opponent troops this pairing caused to become severely wounded."),
      },
      {
        id: "avgDuration",
        name: t("Avg. Battle Duration"),
        value: formatDurationSeconds(avgDurationSeconds),
        description: t("Average battle duration for this pairing."),
      },
      {
        id: "avgTradePercent",
        name: t("Avg. Trade Percentage"),
        value: formatPercent(selectedLoadoutCard.totals.tradePercent),
        description: t(
          "Each battle's kill points gained divided by kill points lost, then averaged across battles."
        ),
      },
      {
        id: "weightedTradePercent",
        name: t("Weighted Trade Percentage"),
        value: formatPercent(selectedLoadoutCard.totals.weightedTradePercent),
        description: t("Total kill points gained divided by total kill points lost."),
      },
      {
        id: "dps",
        name: t("Damage Per Second (DPS)"),
        value: formatPerSecond(
          ratePerSecond(selectedLoadoutCard.totals.dps, selectedLoadoutCard.totals.battleDuration)
        ),
        description: t("Average damage inflicted per second while using this pairing."),
      },
      {
        id: "sps",
        name: t("Sevs Per Second (SPS)"),
        value: formatPerSecond(
          ratePerSecond(selectedLoadoutCard.totals.sps, selectedLoadoutCard.totals.battleDuration)
        ),
        description: t(
          "Average severely wounded troops inflicted per second while using this pairing."
        ),
      },
      {
        id: "tps",
        name: t("Sevs Taken Per Second (TPS)"),
        value: formatPerSecond(
          ratePerSecond(selectedLoadoutCard.totals.tps, selectedLoadoutCard.totals.battleDuration)
        ),
        description: t(
          "Average severely wounded troops taken per second while using this pairing."
        ),
      },
      {
        id: "hps",
        name: t("Healing Per Second (HPS)"),
        value: formatPerSecond(selectedLoadoutCard.totals.hps),
        description: t("Average healing performed per second while using this pairing."),
      },
    ];
  }, [selectedLoadoutCard, t]);

  const opponentGranularity: OpponentGranularity =
    selectedLoadoutKey === ALL_LOADOUT_KEY ? "overall" : loadoutGranularity;
  const opponentLoadoutKey = selectedLoadoutKey === ALL_LOADOUT_KEY ? null : selectedLoadoutKey;
  const canLoadOpponents =
    Boolean(selectedPairing) && loadoutsReady && !pairingsLoading && !pairingsError;

  const {
    data: opponents,
    loading: opponentsLoading,
    error: opponentsError,
  } = usePairingOpponents({
    governorId: activeGovernor?.governorId,
    primaryCommanderId: canLoadOpponents ? (selectedPairing?.primaryCommanderId ?? null) : null,
    secondaryCommanderId: canLoadOpponents ? (selectedPairing?.secondaryCommanderId ?? null) : null,
    granularity: opponentGranularity,
    loadoutKey: opponentLoadoutKey,
    startDate: rangeStartDate,
    endDate: rangeEndDate,
    excludeActivities: excludedActivities,
    excludeBattles: excludedBattles,
  });

  useEffect(() => {
    void opponentsResetKey;
    setShowAllOpponents(false);
  }, [opponentsResetKey]);

  const hasMoreOpponents = opponents.length > 10;
  const visibleOpponents = showAllOpponents ? opponents : opponents.slice(0, 10);
  const opponentRows = useMemo(
    () =>
      visibleOpponents.map((entry, index) => ({
        id: `${entry.enemyPrimaryCommanderId}:${entry.enemySecondaryCommanderId}`,
        index: index + 1,
        pairing: formatCommanderPair(
          entry.enemyPrimaryCommanderId,
          entry.enemySecondaryCommanderId,
          t("Unknown commander")
        ),
        battles: formatNumber(entry.count),
        killPoints: formatNumber(entry.totals.killScore),
        opponentKillPoints: formatNumber(entry.totals.enemyKillScore),
        dps: formatPerSecond(ratePerSecond(entry.totals.dps, entry.totals.battleDuration)),
        sps: formatPerSecond(ratePerSecond(entry.totals.sps, entry.totals.battleDuration)),
        tps: formatPerSecond(ratePerSecond(entry.totals.tps, entry.totals.battleDuration)),
        hps: formatPerSecond(entry.totals.hps),
      })),
    [t, visibleOpponents]
  );

  if (!activeGovernor) {
    return null;
  }

  return (
    <div className="space-y-10">
      <Text>{t("Analyze performance across commander pairings, loadouts, and matchups")}</Text>
      <PairingsFilters
        pairingOptions={pairingOptions}
        pairingValue={selectedPairingKey}
        onPairingChange={setSelectedPairingKey}
        pairingsLoading={pairingsLoading}
        loadoutGranularity={loadoutGranularity}
        onGranularityChange={setLoadoutGranularity}
        startDate={startDate}
        endDate={endDate}
        onStartDateChange={setStartDate}
        onEndDateChange={setEndDate}
        excludedActivities={excludedActivities}
        onExcludedActivitiesChange={setExcludedActivities}
        excludedBattles={excludedBattles}
        onExcludedBattlesChange={setExcludedBattles}
      />
      <PairingsLoadouts
        pairingsLoading={pairingsLoading}
        pairingsError={pairingsError}
        hasSelectedPairing={hasSelectedPairing}
        loadoutsLoading={loadoutsLoading}
        loadoutsError={loadoutsError}
        loadoutCards={loadoutCards}
        selectedLoadoutKey={selectedLoadoutKey}
        onSelectLoadout={(key) => setSelectedLoadoutKey(key)}
      />
      <PairingsLoadoutBreakdown
        pairingsLoading={pairingsLoading}
        pairingsError={pairingsError}
        hasSelectedPairing={hasSelectedPairing}
        loadoutsLoading={loadoutsLoading}
        loadoutsReady={loadoutsReady}
        loadoutsError={loadoutsError}
        hasSelectedLoadout={hasSelectedLoadout}
        generalStats={generalStats}
        enemiesLoading={opponentsLoading}
        enemiesError={opponentsError}
        opponentRows={opponentRows}
        hasMoreOpponents={hasMoreOpponents}
        showAllOpponents={showAllOpponents}
        onToggleShowAllOpponents={() => setShowAllOpponents((prev) => !prev)}
        opponentsId={opponentsId}
      />
    </div>
  );
}
