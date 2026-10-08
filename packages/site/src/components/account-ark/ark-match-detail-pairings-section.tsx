"use client";

import { useExtracted } from "next-intl";
import type { ReactElement } from "react";
import { ArkCommanderPairingCell } from "@/components/account-ark/ark-commander-pairing-cell";
import { ArkSectionHeading, useArkNumber } from "@/components/account-ark/ark-shared";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Text } from "@/components/ui/text";
import type { ArkMatchDetailPairing } from "@/lib/types/ark";

type ArkMatchDetailPairingsSectionProps = { pairings: ArkMatchDetailPairing[] };

export function ArkMatchDetailPairingsSection({
  pairings,
}: ArkMatchDetailPairingsSectionProps): ReactElement {
  const t = useExtracted();
  const number = useArkNumber();

  const rows = pairings
    .map((pairing, index) => ({ ...pairing, index }))
    .toSorted((a, b) => (b.killPoints ?? -1) - (a.killPoints ?? -1));
  const maxPoints = Math.max(
    1,
    ...pairings.flatMap((pairing) => [pairing.killPoints ?? 0, pairing.lossPoints ?? 0])
  );

  return (
    <section className="space-y-4">
      <ArkSectionHeading
        title={t("Pairing performance")}
        description={t("Compare your commander pairings across battles, kills, and trades.")}
      />
      {!rows.length ? (
        <Text>{t("No pairings found for this match.")}</Text>
      ) : (
        <Table
          dense
          className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)] lg:[&_table]:w-full lg:[&_table]:table-fixed lg:[&_table]:min-w-280"
        >
          <TableHead>
            <TableRow>
              <TableHeader className="w-28">{t("Pairing")}</TableHeader>
              <TableHeader className="text-right">{t("Battles")}</TableHeader>
              <TableHeader className="text-right">{t("Battles won")}</TableHeader>
              <TableHeader className="text-right">{t("Win rate")}</TableHeader>
              <TableHeader className="text-right">{t("Kill count")}</TableHeader>
              <TableHeader className="w-44 text-right">{t("Kill points gained")}</TableHeader>
              <TableHeader className="w-44 text-right">{t("Kill points lost")}</TableHeader>
              <TableHeader className="w-44 text-right">{t("Trade percentage")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((pairing) => {
              const winRate =
                pairing.battles != null && pairing.battles > 0 && pairing.battlesWin != null
                  ? (pairing.battlesWin / pairing.battles) * 100
                  : null;
              const tradePercent =
                pairing.lossPoints != null && pairing.lossPoints > 0 && pairing.killPoints != null
                  ? (pairing.killPoints / pairing.lossPoints) * 100
                  : null;

              return (
                <TableRow
                  key={`${pairing.primaryCommanderId}:${pairing.secondaryCommanderId}:${pairing.index}`}
                >
                  <TableCell>
                    <ArkCommanderPairingCell
                      primaryId={pairing.primaryCommanderId}
                      secondaryId={pairing.secondaryCommanderId}
                    />
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {number(pairing.battles)}
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {number(pairing.battlesWin)}
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {winRate == null ? "—" : `${number(winRate)}%`}
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {number(pairing.killCount)}
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {number(pairing.killPoints)}
                    <div
                      aria-hidden="true"
                      className="mt-2 h-1.5 rounded-full bg-zinc-100 dark:bg-zinc-800"
                    >
                      <div
                        className="h-full rounded-full bg-blue-500"
                        style={{ width: `${((pairing.killPoints ?? 0) / maxPoints) * 100}%` }}
                      />
                    </div>
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {number(pairing.lossPoints)}
                    <div
                      aria-hidden="true"
                      className="mt-2 h-1.5 rounded-full bg-zinc-100 dark:bg-zinc-800"
                    >
                      <div
                        className="h-full rounded-full bg-red-500"
                        style={{ width: `${((pairing.lossPoints ?? 0) / maxPoints) * 100}%` }}
                      />
                    </div>
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {tradePercent == null ? "—" : `${number(tradePercent)}%`}
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      )}
    </section>
  );
}
