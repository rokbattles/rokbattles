"use client";

import { useExtracted } from "next-intl";
import { ArkMatchHistoryRow } from "@/components/account-ark/ark-match-history-row";
import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { GameTranslate } from "@/components/v1/game-translate";
import type { ArkMatchRecord } from "@/lib/types/ark";

type ArkMatchHistoryTableProps = {
  rows: ArkMatchRecord[];
};

export function ArkMatchHistoryTable({ rows }: ArkMatchHistoryTableProps) {
  const t = useExtracted();

  return (
    <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
      <TableHead>
        <TableRow>
          <TableHeader className="sm:w-36">{t("Time")}</TableHeader>
          <TableHeader>
            <GameTranslate value="LC_BATTLEFIELD_ISIS" />
          </TableHeader>
          <TableHeader>
            <GameTranslate value="LC_BATTLEFIELD_SETH" />
          </TableHeader>
          <TableHeader className="sm:w-24">{t("Winner")}</TableHeader>
          <TableHeader className="sm:w-24">{t("Members")}</TableHeader>
          <TableHeader className="sm:w-40">{t("Score")}</TableHeader>
        </TableRow>
      </TableHead>
      <TableBody>
        {rows.map((row) => (
          <ArkMatchHistoryRow key={row.matchId} row={row} />
        ))}
      </TableBody>
    </Table>
  );
}
