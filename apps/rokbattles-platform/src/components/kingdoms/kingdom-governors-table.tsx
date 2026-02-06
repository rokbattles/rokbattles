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
import type { KingdomGovernorsPage } from "@/lib/types/kingdom-governor-data";

export default function KingdomGovernorsTable({
  governors,
  offset,
}: {
  governors: KingdomGovernorsPage;
  offset: number;
}) {
  const t = useExtracted();

  return (
    <Table
      className="mt-2 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]"
      dense
      grid
    >
      <TableHead>
        <TableRow>
          <TableHeader className="w-12">{t("#")}</TableHeader>
          <TableHeader className="w-32">{t("ID")}</TableHeader>
          <TableHeader>{t("Name")}</TableHeader>
          <TableHeader className="w-48">{t("Power")}</TableHeader>
        </TableRow>
      </TableHead>
      <TableBody>
        {governors.rows.map((row, index) => (
          <TableRow key={row.id}>
            <TableCell className="tabular-nums">{index + offset + 1}</TableCell>
            <TableCell className="text-zinc-400 tabular-nums">
              {row.governorId}
            </TableCell>
            <TableCell>{row.governorName}</TableCell>
            <TableCell className="tabular-nums">
              {row.power.toLocaleString()}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
