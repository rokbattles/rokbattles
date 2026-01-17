"use client";

import { useTranslations } from "next-intl";
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
  const t = useTranslations("pairings");
  const tCommon = useTranslations("common");
  const tTrends = useTranslations("trends");

  return (
    <section className="space-y-6">
      <Subheading>{t("breakdown.title")}</Subheading>
      {pairingsLoading ? (
        <Text>{t("states.loadingPairings")}</Text>
      ) : pairingsError ? (
        <Text>{pairingsError}</Text>
      ) : !hasSelectedPairing ? (
        <Text>{t("states.selectPairing")}</Text>
      ) : loadoutsLoading || !loadoutsReady ? (
        <Text>{t("breakdown.states.loadingBreakdown")}</Text>
      ) : loadoutsError ? (
        <Text>{loadoutsError}</Text>
      ) : !hasSelectedLoadout ? (
        <Text>{t("breakdown.states.selectLoadout")}</Text>
      ) : (
        <>
          <div className="space-y-3">
            <div className="grid gap-6 grid-cols-2 lg:grid-cols-3">
              {generalStats.map((stat) => (
                <div
                  key={stat.id}
                  className="space-y-3 border-b border-zinc-200/60 pb-4 dark:border-white/10"
                >
                  <div className="space-y-1">
                    <div className="text-sm font-semibold text-zinc-950 dark:text-white">
                      {stat.name}
                    </div>
                    <p className="text-sm text-zinc-600 dark:text-zinc-400">{stat.description}</p>
                  </div>
                  <div className="mt-4 text-3xl/8 font-semibold text-zinc-950 sm:text-3xl dark:text-white">
                    {stat.value}
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="space-y-3">
            <div>
              <div className="text-sm font-semibold text-zinc-950 dark:text-white">
                {t("breakdown.opponents.title")}
              </div>
            </div>
            {enemiesLoading ? (
              <Text>{t("breakdown.opponents.loading")}</Text>
            ) : enemiesError ? (
              <Text>{enemiesError}</Text>
            ) : opponentRows.length === 0 ? (
              <Text>{t("breakdown.opponents.empty")}</Text>
            ) : (
              <>
                <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
                  <TableHead>
                    <TableRow>
                      <TableHeader className="w-12">{tTrends("table.rank")}</TableHeader>
                      <TableHeader>{t("breakdown.opponents.table.pairing")}</TableHeader>
                      <TableHeader className="w-24">{tCommon("labels.battles")}</TableHeader>
                      <TableHeader className="w-32">{tCommon("metrics.killPoints")}</TableHeader>
                      <TableHeader className="w-40">
                        {t("breakdown.stats.enemyKillPoints.label")}
                      </TableHeader>
                      <TableHeader className="w-20">
                        {t("breakdown.opponents.table.dps")}
                      </TableHeader>
                      <TableHeader className="w-20">
                        {t("breakdown.opponents.table.sps")}
                      </TableHeader>
                      <TableHeader className="w-20">
                        {t("breakdown.opponents.table.tps")}
                      </TableHeader>
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
                    {showAllOpponents ? tCommon("actions.showLess") : tCommon("actions.showMore")}
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
