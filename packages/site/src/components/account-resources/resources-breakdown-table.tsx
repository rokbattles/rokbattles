"use client";

import { useExtracted } from "next-intl";
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
import type { ResourceBreakdownRow } from "@/lib/resources/rows";

type ResourcesBreakdownTableProps = {
  rows: ResourceBreakdownRow[];
};

export function ResourcesBreakdownTable({ rows }: ResourcesBreakdownTableProps) {
  const t = useExtracted();

  return (
    <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
      <TableHead>
        <TableRow>
          <TableHeader>{t("Type")}</TableHeader>
          <TableHeader className="lg:w-48">{t("Gain")}</TableHeader>
          <TableHeader className="lg:w-48">{t("Bonus")}</TableHeader>
          <TableHeader className="lg:w-48">{t("Total")}</TableHeader>
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
            <TableCell className="tabular-nums">{formatWholeNumber(row.gain)}</TableCell>
            <TableCell className="tabular-nums">{formatWholeNumber(row.bonus)}</TableCell>
            <TableCell className="tabular-nums">{formatWholeNumber(row.total)}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
