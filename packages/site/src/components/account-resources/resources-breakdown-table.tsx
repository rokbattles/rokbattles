"use client";

import { useExtracted, useFormatter } from "next-intl";
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

type ResourcesBreakdownTableProps = {
  rows: ResourceBreakdownRow[];
};

export function ResourcesBreakdownTable({ rows }: ResourcesBreakdownTableProps): ReactElement {
  const t = useExtracted();
  const intl = useFormatter();

  return (
    <section className="space-y-4">
      <div>
        <Subheading>{t("Gathering breakdown")}</Subheading>
        <Text className="mt-1">
          {t("Base gathering and bonus gains across the selected date range.")}
        </Text>
      </div>
      <Table
        dense
        className="[--gutter:--spacing(4)] lg:[&_table]:w-full lg:[&_table]:min-w-[52rem] lg:[&_table]:table-fixed"
      >
        <caption className="sr-only">{t("Gathering breakdown")}</caption>
        <TableHead>
          <TableRow>
            <TableHeader scope="col">{t("Resource")}</TableHeader>
            <TableHeader scope="col" className="text-right lg:w-40">
              {t("Base gain")}
            </TableHeader>
            <TableHeader scope="col" className="text-right lg:w-40">
              {t("Bonus")}
            </TableHeader>
            <TableHeader scope="col" className="text-right lg:w-40">
              {t("Total")}
            </TableHeader>
            <TableHeader scope="col" className="hidden text-right lg:table-cell lg:w-48">
              {t("Bonus share")}
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
                {intl.number(row.gain, { maximumFractionDigits: 0 })}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {intl.number(row.bonus, { maximumFractionDigits: 0 })}
              </TableCell>
              <TableCell className="text-right font-medium tabular-nums">
                {intl.number(row.total, { maximumFractionDigits: 0 })}
              </TableCell>
              <TableCell className="hidden lg:table-cell">
                <div className="flex items-center justify-end gap-3">
                  <div
                    aria-hidden="true"
                    className="h-1.5 w-20 overflow-hidden rounded-full bg-zinc-100 dark:bg-zinc-800"
                  >
                    <div
                      className="h-full rounded-full bg-blue-500"
                      style={{ width: `${row.total > 0 ? (row.bonus / row.total) * 100 : 0}%` }}
                    />
                  </div>
                  <span className="w-14 text-right tabular-nums">
                    {row.total > 0
                      ? intl.number(row.bonus / row.total, {
                          style: "percent",
                          maximumFractionDigits: 1,
                        })
                      : "—"}
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
