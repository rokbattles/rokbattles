"use client";

import { useExtracted } from "next-intl";
import { SummaryMetric } from "@/components/summary-metric";
import { Button } from "@/components/ui/button";
import { Subheading } from "@/components/ui/heading";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Text } from "@/components/ui/text";

type GeneralStat = {
  id: string;
  name: string;
  value: string;
  description: string;
};

type OpponentRow = {
  id: string;
  index: number;
  pairing: string;
  battles: string;
  killPoints: string;
  opponentKillPoints: string;
  dps: string;
  sps: string;
  tps: string;
  hps: string;
};

type PairingsLoadoutBreakdownProps = {
  pairingsLoading: boolean;
  pairingsError: string | null;
  hasSelectedPairing: boolean;
  loadoutsLoading: boolean;
  loadoutsReady: boolean;
  loadoutsError: string | null;
  hasSelectedLoadout: boolean;
  generalStats: GeneralStat[];
  enemiesLoading: boolean;
  enemiesError: string | null;
  opponentRows: OpponentRow[];
  hasMoreOpponents: boolean;
  showAllOpponents: boolean;
  onToggleShowAllOpponents: () => void;
  opponentsId: string;
};

export function PairingsLoadoutBreakdown({
  pairingsLoading,
  pairingsError,
  hasSelectedPairing,
  loadoutsLoading,
  loadoutsReady,
  loadoutsError,
  hasSelectedLoadout,
  generalStats,
  enemiesLoading,
  enemiesError,
  opponentRows,
  hasMoreOpponents,
  showAllOpponents,
  onToggleShowAllOpponents,
  opponentsId,
}: PairingsLoadoutBreakdownProps) {
  const t = useExtracted();

  return (
    <section className="space-y-6">
      <Subheading>{t("Loadout breakdown")}</Subheading>
      {pairingsLoading ? (
        <Text>{t("Loading pairings...")}</Text>
      ) : pairingsError ? (
        <Text>{pairingsError}</Text>
      ) : !hasSelectedPairing ? (
        <Text>{t("Select a pairing to get started.")}</Text>
      ) : loadoutsLoading || !loadoutsReady ? (
        <Text>{t("Loading loadout breakdown...")}</Text>
      ) : loadoutsError ? (
        <Text>{loadoutsError}</Text>
      ) : !hasSelectedLoadout ? (
        <Text>{t("Select a loadout to view the breakdown.")}</Text>
      ) : (
        <>
          <div className="space-y-3">
            <div className="grid grid-cols-2 gap-6 lg:grid-cols-3">
              {generalStats.map((stat) => (
                <SummaryMetric
                  key={stat.id}
                  description={stat.description}
                  label={stat.name}
                  value={stat.value}
                />
              ))}
            </div>
          </div>

          <div className="space-y-3">
            <div>
              <div className="text-sm font-semibold text-zinc-950 dark:text-white">
                {t("Opponent pairings")}
              </div>
            </div>
            {enemiesLoading ? (
              <Text>{t("Loading enemy matchups...")}</Text>
            ) : enemiesError ? (
              <Text>{enemiesError}</Text>
            ) : opponentRows.length === 0 ? (
              <Text>{t("No enemy pairings found for this selection.")}</Text>
            ) : (
              <>
                <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
                  <TableHead>
                    <TableRow>
                      <TableHeader className="w-12">{t("#")}</TableHeader>
                      <TableHeader>{t("Opponent pairing")}</TableHeader>
                      <TableHeader className="w-24">{t("Battles")}</TableHeader>
                      <TableHeader className="w-32">{t("Kill Points")}</TableHeader>
                      <TableHeader className="w-40">{t("Opponent Kill Points")}</TableHeader>
                      <TableHeader className="w-20">{t("DPS")}</TableHeader>
                      <TableHeader className="w-20">{t("SPS")}</TableHeader>
                      <TableHeader className="w-20">{t("TPS")}</TableHeader>
                      <TableHeader className="w-20">{t("HPS")}</TableHeader>
                    </TableRow>
                  </TableHead>
                  <TableBody id={opponentsId}>
                    {opponentRows.map((entry) => (
                      <TableRow key={entry.id}>
                        <TableCell className="w-12 tabular-nums text-zinc-500 dark:text-zinc-400">
                          {entry.index}
                        </TableCell>
                        <TableCell className="text-zinc-900 dark:text-white">
                          {entry.pairing}
                        </TableCell>
                        <TableCell className="w-24">{entry.battles}</TableCell>
                        <TableCell className="w-32">{entry.killPoints}</TableCell>
                        <TableCell className="w-40">{entry.opponentKillPoints}</TableCell>
                        <TableCell className="w-20">{entry.dps}</TableCell>
                        <TableCell className="w-20">{entry.sps}</TableCell>
                        <TableCell className="w-20">{entry.tps}</TableCell>
                        <TableCell className="w-20">{entry.hps}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
                {hasMoreOpponents ? (
                  <Button
                    plain
                    type="button"
                    onClick={onToggleShowAllOpponents}
                    aria-expanded={showAllOpponents}
                    aria-controls={opponentsId}
                    className="text-sm"
                  >
                    {showAllOpponents ? t("Show less") : t("Show more")}
                  </Button>
                ) : null}
              </>
            )}
          </div>
        </>
      )}
    </section>
  );
}
