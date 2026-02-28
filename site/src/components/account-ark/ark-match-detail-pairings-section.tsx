"use client";

import { useExtracted } from "next-intl";
import { ArkCommanderPairingCell } from "@/components/account-ark/ark-commander-pairing-cell";
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
import { formatArkBattlesValue, formatArkMetricValue } from "@/lib/ark/detail-format";
import type { ArkMatchDetailPairing } from "@/lib/types/ark";

type ArkMatchDetailPairingsSectionProps = {
  pairings: ArkMatchDetailPairing[];
};

export function ArkMatchDetailPairingsSection({ pairings }: ArkMatchDetailPairingsSectionProps) {
  const t = useExtracted();
  const unavailableLabel = t("N/A");

  return (
    <div className="space-y-2">
      <Subheading>{t("Pairings Performance")}</Subheading>
      {pairings.length === 0 ? (
        <Text>{t("No pairings found for this match.")}</Text>
      ) : (
        <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
          <TableHead>
            <TableRow>
              <TableHeader className="w-12">{t("#")}</TableHeader>
              <TableHeader>{t("Pairing")}</TableHeader>
              <TableHeader className="w-40">{t("Battles (win/total)")}</TableHeader>
              <TableHeader className="w-40">{t("Kill Count")}</TableHeader>
              <TableHeader className="w-40">{t("Kill Points")}</TableHeader>
              <TableHeader className="w-40">{t("Severely Wounded")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {pairings.map((pairing, index) => (
              <TableRow
                key={`${pairing.primaryCommanderId ?? "na"}-${pairing.secondaryCommanderId ?? "na"}-${index}`}
              >
                <TableCell className="w-12 tabular-nums text-zinc-500 dark:text-zinc-400">
                  {index + 1}
                </TableCell>
                <TableCell>
                  <ArkCommanderPairingCell
                    primaryId={pairing.primaryCommanderId}
                    secondaryId={pairing.secondaryCommanderId}
                  />
                </TableCell>
                <TableCell className="tabular-nums">
                  {formatArkBattlesValue(pairing.battlesWin, pairing.battles, unavailableLabel)}
                </TableCell>
                <TableCell className="tabular-nums">
                  {formatArkMetricValue(pairing.killCount, unavailableLabel)}
                </TableCell>
                <TableCell className="tabular-nums">
                  {formatArkMetricValue(pairing.killPoints, unavailableLabel)}
                </TableCell>
                <TableCell className="tabular-nums">
                  {formatArkMetricValue(pairing.severelyWounded, unavailableLabel)}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </div>
  );
}
