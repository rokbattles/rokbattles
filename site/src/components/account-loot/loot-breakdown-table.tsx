"use client";

import { useExtracted } from "next-intl";
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
  const t = useExtracted();

  return (
    <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
      <TableHead>
        <TableRow>
          <TableHeader>{t("Name")}</TableHeader>
          <TableHeader className="lg:w-36">{t("Amount")}</TableHeader>
          <TableHeader className="lg:w-36">{t("Drops")}</TableHeader>
        </TableRow>
      </TableHead>
      <TableBody>
        {rows.map((row) => (
          <TableRow key={row.key}>
            <TableCell>{row.name}</TableCell>
            <TableCell className="tabular-nums">{formatWholeNumber(row.total)}</TableCell>
            <TableCell className="tabular-nums">{formatWholeNumber(row.count)}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
