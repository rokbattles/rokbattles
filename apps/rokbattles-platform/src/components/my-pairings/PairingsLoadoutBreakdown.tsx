"use client";

import { Button } from "@/components/ui/Button";
import { Subheading } from "@/components/ui/Heading";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/Table";
import { Text } from "@/components/ui/Text";

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
  return (
    <section className="space-y-6">
      <Subheading>Loadout breakdown</Subheading>
      {pairingsLoading ? (
        <Text>Loading pairings...</Text>
      ) : pairingsError ? (
        <Text>{pairingsError}</Text>
      ) : !hasSelectedPairing ? (
        <Text>Select a pairing to get started.</Text>
      ) : loadoutsLoading || !loadoutsReady ? (
        <Text>Loading loadout breakdown...</Text>
      ) : loadoutsError ? (
        <Text>{loadoutsError}</Text>
      ) : !hasSelectedLoadout ? (
        <Text>Select a loadout to view the breakdown.</Text>
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
                Opponent pairings
              </div>
            </div>
            {enemiesLoading ? (
              <Text>Loading enemy matchups...</Text>
            ) : enemiesError ? (
              <Text>{enemiesError}</Text>
            ) : opponentRows.length === 0 ? (
              <Text>No enemy pairings found for this selection.</Text>
            ) : (
              <>
                <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
                  <TableHead>
                    <TableRow>
                      <TableHeader className="w-12">#</TableHeader>
                      <TableHeader>Opponent pairing</TableHeader>
                      <TableHeader className="w-24">Battles</TableHeader>
                      <TableHeader className="w-32">Kill Points</TableHeader>
                      <TableHeader className="w-40">Opponent Kill Points</TableHeader>
                      <TableHeader className="w-20">DPS</TableHeader>
                      <TableHeader className="w-20">SPS</TableHeader>
                      <TableHeader className="w-20">TPS</TableHeader>
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
                    {showAllOpponents ? "Show less" : "Show more"}
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
