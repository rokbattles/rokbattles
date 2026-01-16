"use client";

import { useTranslations } from "next-intl";
import { useContext, useEffect, useId, useMemo, useState } from "react";
import { PairingsFilters } from "@/components/my-pairings/pairings-filters";
import { PairingsLoadoutBreakdown } from "@/components/my-pairings/pairings-loadout-breakdown";
import {
  type LoadoutCard,
  PairingsLoadouts,
} from "@/components/my-pairings/pairings-loadouts";
import { Text } from "@/components/ui/text";
import { getCommanderName } from "@/hooks/use-commander-name";
import {
  type EnemyGranularity,
  type LoadoutGranularity,
  type LoadoutSnapshot,
  usePairingEnemies,
  usePairingLoadouts,
  usePairings,
} from "@/hooks/use-pairings";
import { formatDurationShort } from "@/lib/datetime";
import { GovernorContext } from "@/providers/governor-context";

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
  if (
    !(Number.isFinite(value) && Number.isFinite(durationMillis)) ||
    durationMillis <= 0
  ) {
    return 0;
  }

  return value / (durationMillis / 1000);
}

function createPairingKey(primaryId: number, secondaryId: number) {
  return `${primaryId}:${secondaryId}`;
}

function formatCommanderPair(
  primaryId: number,
  secondaryId: number,
  unknownLabel: string
) {
  const primaryName =
    primaryId > 0 ? (getCommanderName(primaryId) ?? primaryId) : unknownLabel;
  const secondaryName =
    secondaryId > 0 ? (getCommanderName(secondaryId) ?? secondaryId) : null;

  if (!secondaryName) {
    return String(primaryName);
  }

  return `${primaryName} / ${secondaryName}`;
}

export function MyPairingsContent() {
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("My Pairings must be used within a GovernorProvider");
  }

  const t = useTranslations("pairings");
  const tCommon = useTranslations("common");
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
  const [selectedPairingKey, setSelectedPairingKey] = useState<string | null>(
    null
  );
  const [loadoutGranularity, setLoadoutGranularity] =
    useState<LoadoutGranularity>("normalized");
  const [selectedLoadoutKey, setSelectedLoadoutKey] = useState<string | null>(
    ALL_LOADOUT_KEY
  );
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
            createPairingKey(
              pairing.primaryCommanderId,
              pairing.secondaryCommanderId
            ) === current
        )
      ) {
        return current;
      }

      const first = data[0];
      return createPairingKey(
        first.primaryCommanderId,
        first.secondaryCommanderId
      );
    });
  }, [data]);

  const pairingOptions = useMemo(
    () =>
      data.map((pairing) => ({
        value: createPairingKey(
          pairing.primaryCommanderId,
          pairing.secondaryCommanderId
        ),
        label: formatCommanderPair(
          pairing.primaryCommanderId,
          pairing.secondaryCommanderId,
          tCommon("labels.unknownCommander")
        ),
      })),
    [data, tCommon]
  );

  const selectedPairing = data.find(
    (pairing) =>
      createPairingKey(
        pairing.primaryCommanderId,
        pairing.secondaryCommanderId
      ) === selectedPairingKey
  );
  const hasSelectedPairing = Boolean(selectedPairing);

  const canLoadLoadouts =
    hasSelectedPairing && !pairingsLoading && !pairingsError;
  const {
    data: loadouts,
    loading: loadoutsLoading,
    error: loadoutsError,
  } = usePairingLoadouts({
    governorId: activeGovernor?.governorId,
    primaryCommanderId: canLoadLoadouts
      ? (selectedPairing?.primaryCommanderId ?? null)
      : null,
    secondaryCommanderId: canLoadLoadouts
      ? (selectedPairing?.secondaryCommanderId ?? null)
      : null,
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
    [
      selectedPairingKey,
      selectedLoadoutKey,
      loadoutGranularity,
      rangeStartDate,
      rangeEndDate,
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
      label: t("labels.allLoadouts"),
      count: selectedPairing.count,
      totals: selectedPairing.totals,
      loadout: EMPTY_LOADOUT,
    };

    const cards = loadouts.map<LoadoutCard>((loadout, index) => ({
      ...loadout,
      label: t("labels.loadout", { index: index + 1 }),
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
      selectedLoadoutCard.count > 0
        ? durationSeconds / selectedLoadoutCard.count
        : 0;

    return [
      {
        id: "battles",
        name: tCommon("labels.battles"),
        value: formatNumber(selectedLoadoutCard.count),
        description: t("breakdown.stats.battles.description"),
      },
      {
        id: "killPoints",
        name: tCommon("metrics.killPoints"),
        value: formatNumber(selectedLoadoutCard.totals.killScore),
        description: t("breakdown.stats.killPoints.description"),
      },
      {
        id: "enemyKillPoints",
        name: t("breakdown.stats.enemyKillPoints.label"),
        value: formatNumber(selectedLoadoutCard.totals.enemyKillScore),
        description: t("breakdown.stats.enemyKillPoints.description"),
      },
      {
        id: "severelyWounded",
        name: t("breakdown.stats.severelyWounded.label"),
        value: formatNumber(selectedLoadoutCard.totals.severelyWounded),
        description: t("breakdown.stats.severelyWounded.description"),
      },
      {
        id: "enemySeverelyWounded",
        name: t("breakdown.stats.enemySeverelyWounded.label"),
        value: formatNumber(selectedLoadoutCard.totals.enemySeverelyWounded),
        description: t("breakdown.stats.enemySeverelyWounded.description"),
      },
      {
        id: "avgDuration",
        name: t("breakdown.stats.avgDuration.label"),
        value: formatDurationSeconds(avgDurationSeconds),
        description: t("breakdown.stats.avgDuration.description"),
      },
      {
        id: "dps",
        name: t("breakdown.stats.dps.label"),
        value: formatPerSecond(
          ratePerSecond(
            selectedLoadoutCard.totals.dps,
            selectedLoadoutCard.totals.battleDuration
          )
        ),
        description: t("breakdown.stats.dps.description"),
      },
      {
        id: "sps",
        name: t("breakdown.stats.sps.label"),
        value: formatPerSecond(
          ratePerSecond(
            selectedLoadoutCard.totals.sps,
            selectedLoadoutCard.totals.battleDuration
          )
        ),
        description: t("breakdown.stats.sps.description"),
      },
      {
        id: "tps",
        name: t("breakdown.stats.tps.label"),
        value: formatPerSecond(
          ratePerSecond(
            selectedLoadoutCard.totals.tps,
            selectedLoadoutCard.totals.battleDuration
          )
        ),
        description: t("breakdown.stats.tps.description"),
      },
    ];
  }, [selectedLoadoutCard, t, tCommon]);

  const enemyGranularity: EnemyGranularity =
    selectedLoadoutKey === ALL_LOADOUT_KEY ? "overall" : loadoutGranularity;
  const enemyLoadoutKey =
    selectedLoadoutKey === ALL_LOADOUT_KEY ? null : selectedLoadoutKey;
  const canLoadEnemies =
    Boolean(selectedPairing) &&
    loadoutsReady &&
    !pairingsLoading &&
    !pairingsError;

  const {
    data: enemies,
    loading: enemiesLoading,
    error: enemiesError,
  } = usePairingEnemies({
    governorId: activeGovernor?.governorId,
    primaryCommanderId: canLoadEnemies
      ? (selectedPairing?.primaryCommanderId ?? null)
      : null,
    secondaryCommanderId: canLoadEnemies
      ? (selectedPairing?.secondaryCommanderId ?? null)
      : null,
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
          tCommon("labels.unknownCommander")
        ),
        battles: formatNumber(entry.count),
        killPoints: formatNumber(entry.totals.killScore),
        opponentKillPoints: formatNumber(entry.totals.enemyKillScore),
        dps: formatPerSecond(
          ratePerSecond(entry.totals.dps, entry.totals.battleDuration)
        ),
        sps: formatPerSecond(
          ratePerSecond(entry.totals.sps, entry.totals.battleDuration)
        ),
        tps: formatPerSecond(
          ratePerSecond(entry.totals.tps, entry.totals.battleDuration)
        ),
      })),
    [tCommon, visibleOpponents]
  );

  if (!activeGovernor) {
    return null;
  }

  return (
    <div className="space-y-10">
      <Text>{t("intro")}</Text>
      <PairingsFilters
        endDate={endDate}
        loadoutGranularity={loadoutGranularity}
        onEndDateChange={setEndDate}
        onGranularityChange={setLoadoutGranularity}
        onPairingChange={setSelectedPairingKey}
        onStartDateChange={setStartDate}
        pairingOptions={pairingOptions}
        pairingsLoading={pairingsLoading}
        pairingValue={selectedPairingKey}
        startDate={startDate}
      />
      <PairingsLoadouts
        hasSelectedPairing={hasSelectedPairing}
        loadoutCards={loadoutCards}
        loadoutsError={loadoutsError}
        loadoutsLoading={loadoutsLoading}
        onSelectLoadout={(key) => setSelectedLoadoutKey(key)}
        pairingsError={pairingsError}
        pairingsLoading={pairingsLoading}
        selectedLoadoutKey={selectedLoadoutKey}
      />
      <PairingsLoadoutBreakdown
        enemiesError={enemiesError}
        enemiesLoading={enemiesLoading}
        generalStats={generalStats}
        hasMoreOpponents={hasMoreOpponents}
        hasSelectedLoadout={hasSelectedLoadout}
        hasSelectedPairing={hasSelectedPairing}
        loadoutsError={loadoutsError}
        loadoutsLoading={loadoutsLoading}
        loadoutsReady={loadoutsReady}
        onToggleShowAllOpponents={() => setShowAllOpponents((prev) => !prev)}
        opponentRows={opponentRows}
        opponentsId={opponentsId}
        pairingsError={pairingsError}
        pairingsLoading={pairingsLoading}
        showAllOpponents={showAllOpponents}
      />
    </div>
  );
}
