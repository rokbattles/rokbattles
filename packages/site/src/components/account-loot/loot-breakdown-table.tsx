"use client";

import { LootIcon } from "@/components/account-loot/loot-icon";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { formatWholeNumber } from "@/lib/loot/format";
import type { LootRewardRow } from "@/lib/loot/reward-rows";

type LootBreakdownTableProps = {
  rows: LootRewardRow[];
};

export function LootBreakdownTable({ rows }: LootBreakdownTableProps) {
  return (
    <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
      <TableHead>
        <TableRow>
          <TableHeader>Name</TableHeader>
          <TableHeader className="w-28">Amount</TableHeader>
          <TableHeader className="w-24">Seen</TableHeader>
        </TableRow>
      </TableHead>
      <TableBody>
        {rows.map((row) => (
          <TableRow key={row.key}>
            <TableCell>
              <div className="flex items-center gap-3">
                <LootIcon spriteUrls={row.spriteUrls} />
                <span>{row.name}</span>
              </div>
            </TableCell>
            <TableCell className="w-28 tabular-nums">{formatWholeNumber(row.total)}</TableCell>
            <TableCell className="w-24 tabular-nums">{formatWholeNumber(row.count)}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
