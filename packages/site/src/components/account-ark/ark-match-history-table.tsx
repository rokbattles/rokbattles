"use client";

import { useExtracted } from "next-intl";
import type { ReactElement } from "react";
import { ArkMatchHistoryRow } from "@/components/account-ark/ark-match-history-row";
import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import type { ArkMatchRecord } from "@/lib/types/ark";

type ArkMatchHistoryTableProps = { rows: ArkMatchRecord[] };

export function ArkMatchHistoryTable({ rows }: ArkMatchHistoryTableProps): ReactElement {
  const t = useExtracted();

  return (
    <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
      <TableHead>
        <TableRow>
          <TableHeader className="w-40">{t("Date")}</TableHeader>
          <TableHeader>{t("Matchup")}</TableHeader>
          <TableHeader className="w-44">{t("League")}</TableHeader>
          <TableHeader className="w-24">{t("Result")}</TableHeader>
          <TableHeader className="w-48 text-right">{t("Alliance score")}</TableHeader>
          <TableHeader className="w-32 text-right">{t("Your score")}</TableHeader>
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
