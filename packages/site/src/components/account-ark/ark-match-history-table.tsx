"use client";

import type { ReactElement } from "react";
import { ArkMatchHistoryRow } from "@/components/account-ark/ark-match-history-row";
import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import type { ArkMatchRecord } from "@/lib/types/ark";

type ArkMatchHistoryTableProps = { rows: ArkMatchRecord[] };

export function ArkMatchHistoryTable({ rows }: ArkMatchHistoryTableProps): ReactElement {
  return (
    <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
      <TableHead>
        <TableRow>
          <TableHeader className="w-40">Date</TableHeader>
          <TableHeader>Matchup</TableHeader>
          <TableHeader className="w-44">League</TableHeader>
          <TableHeader className="w-24">Result</TableHeader>
          <TableHeader className="w-48 text-right">Alliance score</TableHeader>
          <TableHeader className="w-32 text-right">Your score</TableHeader>
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
