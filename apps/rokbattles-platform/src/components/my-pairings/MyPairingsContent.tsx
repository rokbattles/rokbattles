"use client";

import { useContext, useEffect, useId, useMemo, useState } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";
import { PairingsFilters } from "@/components/my-pairings/PairingsFilters";
import { PairingsLoadoutBreakdown } from "@/components/my-pairings/PairingsLoadoutBreakdown";
import { type LoadoutCard, PairingsLoadouts } from "@/components/my-pairings/PairingsLoadouts";
import { Text } from "@/components/ui/Text";
import { getCommanderName } from "@/hooks/useCommanderName";
import {
  type EnemyGranularity,
  type LoadoutGranularity,
  type LoadoutSnapshot,
  usePairingEnemies,
  usePairingLoadouts,
  usePairings,
} from "@/hooks/usePairings";
import { formatDurationShort } from "@/lib/datetime";

const numberFormatter = new Intl.NumberFormat("en-US");
const perSecondFormatter = new Intl.NumberFormat("en-US", {
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

function formatPerSecond(value: number): string {
  if (!Number.isFinite(value)) {
    return "0/s";
  }

  return `${perSecondFormatter.format(value)}/s`;
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

function createLoadoutLabel(index: number) {
  return `Loadout ${index + 1}`;
}

export function MyPairingsContent() {
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("My Pairings must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;

  const [startDate, setStartDate] = useState<string>("");
  const [endDate, setEndDate] = useState<string>("");
  const hasCustomRange = Boolean(startDate && endDate);
  const rangeStartDate = hasCustomRange ? startDate : undefined;
  const rangeEndDate = hasCustomRange ? endDate : undefined;
  const {
    data,
    loading: pairingsLoading,
    error: pairingsError,
    year,
  } = usePairings({
    governorId: activeGovernor?.governorId,
    startDate: rangeStartDate,
    endDate: rangeEndDate,
  });
  const [selectedPairingKey, setSelectedPairingKey] = useState<string | null>(null);
  const [loadoutGranularity, setLoadoutGranularity] = useState<LoadoutGranularity>("normalized");
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
          "Unknown commander"
        ),
      })),
    [data]
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
    year,
    startDate: rangeStartDate,
    endDate: rangeEndDate,
  });
  const loadoutsResetKey = useMemo(
    () =>
      [
        activeGovernor?.governorId ?? "none",
        selectedPairingKey ?? "none",
        loadoutGranularity,
        rangeStartDate ?? "none",
        rangeEndDate ?? "none",
        canLoadLoadouts ? "ready" : "idle",
      ].join("|"),
    [
      activeGovernor?.governorId,
      selectedPairingKey,
      loadoutGranularity,
      rangeStartDate,
      rangeEndDate,
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
      ].join("|"),
    [selectedPairingKey, selectedLoadoutKey, loadoutGranularity, rangeStartDate, rangeEndDate]
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
      label: "All loadouts",
      count: selectedPairing.count,
      totals: selectedPairing.totals,
      loadout: EMPTY_LOADOUT,
    };

    const cards = loadouts.map<LoadoutCard>((loadout, index) => ({
      ...loadout,
      label: createLoadoutLabel(index),
    }));

    return [allLoadouts, ...cards];
  }, [loadouts, selectedPairing]);

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
        name: "Battles",
        value: formatNumber(selectedLoadoutCard.count),
        description: "Total battles recorded for this loadout.",
      },
      {
        id: "killPoints",
        name: "Kill Points",
        value: formatNumber(selectedLoadoutCard.totals.killScore),
        description: "Total kill points earned while using this pairing.",
      },
      {
        id: "enemyKillPoints",
        name: "Opponent Kill Points",
        value: formatNumber(selectedLoadoutCard.totals.enemyKillScore),
        description: "Total kill points your opponents earned against you.",
      },
      {
        id: "severelyWounded",
        name: "Severely Wounded (Taken)",
        value: formatNumber(selectedLoadoutCard.totals.severelyWounded),
        description: "Number of your troops that became severely wounded while using this pairing.",
      },
      {
        id: "enemySeverelyWounded",
        name: "Severely Wounded (Inflicted)",
        value: formatNumber(selectedLoadoutCard.totals.enemySeverelyWounded),
        description: "Number of opponent troops you caused to become severely wounded.",
      },
      {
        id: "avgDuration",
        name: "Avg. Battle Duration",
        value: formatDurationSeconds(avgDurationSeconds),
        description: "Average duration of battles recorded while using this pairing.",
      },
      {
        id: "dps",
        name: "Damage Per Second (DPS)",
        value: formatPerSecond(
          ratePerSecond(selectedLoadoutCard.totals.dps, selectedLoadoutCard.totals.battleDuration)
        ),
        description: "Average amount of damage you inflict per second while using this pairing.",
      },
      {
        id: "sps",
        name: "Sevs Per Second (SPS)",
        value: formatPerSecond(
          ratePerSecond(selectedLoadoutCard.totals.sps, selectedLoadoutCard.totals.battleDuration)
        ),
        description: "Rate at which you inflict severely wounded troops each second.",
      },
      {
        id: "tps",
        name: "Sevs Taken Per Second (TPS)",
        value: formatPerSecond(
          ratePerSecond(selectedLoadoutCard.totals.tps, selectedLoadoutCard.totals.battleDuration)
        ),
        description: "Rate at which your troops become severely wounded each second.",
      },
    ];
  }, [selectedLoadoutCard]);

  const enemyGranularity: EnemyGranularity =
    selectedLoadoutKey === ALL_LOADOUT_KEY ? "overall" : loadoutGranularity;
  const enemyLoadoutKey = selectedLoadoutKey === ALL_LOADOUT_KEY ? null : selectedLoadoutKey;
  const canLoadEnemies =
    Boolean(selectedPairing) && loadoutsReady && !pairingsLoading && !pairingsError;

  const {
    data: enemies,
    loading: enemiesLoading,
    error: enemiesError,
  } = usePairingEnemies({
    governorId: activeGovernor?.governorId,
    primaryCommanderId: canLoadEnemies ? (selectedPairing?.primaryCommanderId ?? null) : null,
    secondaryCommanderId: canLoadEnemies ? (selectedPairing?.secondaryCommanderId ?? null) : null,
    granularity: enemyGranularity,
    loadoutKey: enemyLoadoutKey,
    year,
    startDate: rangeStartDate,
    endDate: rangeEndDate,
  });

  useEffect(() => {
    void opponentsResetKey;
    setShowAllOpponents(false);
  }, [opponentsResetKey]);

  const hasMoreOpponents = enemies.length > 10;
  const visibleOpponents = showAllOpponents ? enemies : enemies.slice(0, 10);
  const opponentRows = useMemo(
    () =>
      visibleOpponents.map((entry, index) => ({
        id: `${entry.enemyPrimaryCommanderId}:${entry.enemySecondaryCommanderId}`,
        index: index + 1,
        pairing: formatCommanderPair(
          entry.enemyPrimaryCommanderId,
          entry.enemySecondaryCommanderId,
          "Unknown commander"
        ),
        battles: formatNumber(entry.count),
        killPoints: formatNumber(entry.totals.killScore),
        opponentKillPoints: formatNumber(entry.totals.enemyKillScore),
        dps: formatPerSecond(ratePerSecond(entry.totals.dps, entry.totals.battleDuration)),
        sps: formatPerSecond(ratePerSecond(entry.totals.sps, entry.totals.battleDuration)),
        tps: formatPerSecond(ratePerSecond(entry.totals.tps, entry.totals.battleDuration)),
      })),
    [visibleOpponents]
  );

  if (!activeGovernor) {
    return null;
  }

  return (
    <div className="space-y-10">
      <Text>Analyze performance across commander pairings, loadouts, and matchups</Text>
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
        enemiesLoading={enemiesLoading}
        enemiesError={enemiesError}
        opponentRows={opponentRows}
        hasMoreOpponents={hasMoreOpponents}
        showAllOpponents={showAllOpponents}
        onToggleShowAllOpponents={() => setShowAllOpponents((prev) => !prev)}
        opponentsId={opponentsId}
      />
    </div>
  );
}
