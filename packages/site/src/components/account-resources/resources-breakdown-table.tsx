"use client";

import type { ReactElement } from "react";
import { LootIcon } from "@/components/account-loot/loot-icon";
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
import type { ResourceBreakdownRow } from "@/lib/resources/rows";

const integerFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });
const percentFormatter = new Intl.NumberFormat("en-US", {
  style: "percent",
  maximumFractionDigits: 1,
});

type ResourcesBreakdownTableProps = {
  rows: ResourceBreakdownRow[];
};

export function ResourcesBreakdownTable({ rows }: ResourcesBreakdownTableProps): ReactElement {
  return (
    <section className="space-y-4">
      <div>
        <Subheading>Gathering breakdown</Subheading>
        <Text className="mt-1">Base gathering and bonus gains across the selected date range.</Text>
      </div>
      <Table
        dense
        className="[--gutter:--spacing(4)] lg:[&_table]:w-full lg:[&_table]:min-w-[52rem] lg:[&_table]:table-fixed"
      >
        <caption className="sr-only">Gathering breakdown</caption>
        <TableHead>
          <TableRow>
            <TableHeader scope="col">Resource</TableHeader>
            <TableHeader scope="col" className="text-right lg:w-40">
              Base gain
            </TableHeader>
            <TableHeader scope="col" className="text-right lg:w-40">
              Bonus
            </TableHeader>
            <TableHeader scope="col" className="text-right lg:w-40">
              Total
            </TableHeader>
            <TableHeader scope="col" className="hidden text-right lg:table-cell lg:w-48">
              Bonus share
            </TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          {rows.map((row) => (
            <TableRow key={row.key}>
              <TableCell>
                <div className="flex items-center gap-3">
                  <LootIcon spriteUrls={row.spriteUrls} />
                  {row.name}
                </div>
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {integerFormatter.format(row.gain)}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {integerFormatter.format(row.bonus)}
              </TableCell>
              <TableCell className="text-right font-medium tabular-nums">
                {integerFormatter.format(row.total)}
              </TableCell>
              <TableCell className="hidden lg:table-cell">
                <div className="flex items-center justify-end gap-3">
                  <div
                    aria-hidden="true"
                    className="h-1.5 w-20 overflow-hidden rounded-full bg-zinc-800"
                  >
                    <div
                      className="h-full rounded-full bg-blue-500"
                      style={{ width: `${row.total > 0 ? (row.bonus / row.total) * 100 : 0}%` }}
                    />
                  </div>
                  <span className="w-14 text-right tabular-nums">
                    {row.total > 0 ? percentFormatter.format(row.bonus / row.total) : "—"}
                  </span>
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </section>
  );
}
