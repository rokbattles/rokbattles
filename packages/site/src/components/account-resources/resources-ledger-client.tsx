"use client";

import { ArrowDown, ArrowUp, Download } from "lucide-react";
import { useExtracted, useFormatter } from "next-intl";
import { type CSSProperties, type ReactElement, useState } from "react";
import { Button } from "@/components/ui/button";
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
import { type ResourceDay, resourceLedgerCsv } from "@/lib/resources/analytics";
import type { ResourceKey } from "@/lib/resources/catalog";
import type { ResourceBreakdownRow } from "@/lib/resources/rows";

const PAGE_SIZE = 15;

type ResourcesLedgerClientProps = {
  days: ResourceDay[];
  rows: ResourceBreakdownRow[];
  governorId: number;
};

type LedgerSortKey = "date" | "reports" | ResourceKey;

type LedgerSort = {
  key: LedgerSortKey;
  descending: boolean;
};

export function ResourcesLedgerClient({
  days,
  rows,
  governorId,
}: ResourcesLedgerClientProps): ReactElement {
  const t = useExtracted();
  const intl = useFormatter();
  const [requestedSort, setSort] = useState<LedgerSort>({ key: "date", descending: true });
  const [page, setPage] = useState(0);
  const [onlyReported, setOnlyReported] = useState(false);

  const headers: { key: LedgerSortKey; name: string }[] = [
    { key: "date", name: t("Day (UTC)") },
    { key: "reports", name: t("Reports") },
    ...rows,
  ];
  const sort: LedgerSort = headers.some((column) => column.key === requestedSort.key)
    ? requestedSort
    : { key: "date", descending: true };
  const sortDirection = sort.descending ? "descending" : "ascending";
  const SortIcon = sort.descending ? ArrowDown : ArrowUp;

  const visibleDays = days
    .filter((day) => !onlyReported || day.reports > 0)
    .sort((a, b) => {
      let difference: number;

      switch (sort.key) {
        case "date":
          difference = a.date.localeCompare(b.date);
          break;
        case "reports":
          difference = a.reports - b.reports;
          break;
        default:
          difference = a.values[sort.key] - b.values[sort.key];
      }

      return (sort.descending ? -difference : difference) || b.date.localeCompare(a.date);
    });

  const pages = Math.max(1, Math.ceil(visibleDays.length / PAGE_SIZE));
  const currentPage = Math.min(page, pages - 1);
  const pageDays = visibleDays.slice(currentPage * PAGE_SIZE, (currentPage + 1) * PAGE_SIZE);

  const exportCsv = (): void => {
    const blob = new Blob(["\uFEFF", resourceLedgerCsv(visibleDays)], {
      type: "text/csv;charset=utf-8;",
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");

    link.href = url;
    link.download = `resources-${governorId}-${days[0]?.date}-${days.at(-1)?.date}.csv`;
    link.click();

    URL.revokeObjectURL(url);
  };

  return (
    <section className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <Subheading>{t("Gathering ledger")}</Subheading>
          <Text className="mt-1">
            {t("Exact totals for selected resources. Report counts include all resources.")}
          </Text>
        </div>
        <Button outline onClick={exportCsv} disabled={visibleDays.length === 0}>
          <Download data-slot="icon" />
          {t("Export CSV")}
        </Button>
      </div>
      <label className="flex w-fit items-center gap-2 text-sm text-zinc-600 dark:text-zinc-400">
        <input
          type="checkbox"
          checked={onlyReported}
          className="size-4 accent-zinc-700"
          onChange={(event) => {
            setOnlyReported(event.target.checked);
            setPage(0);
          }}
        />
        {t("Only days with reports")}
      </label>
      <Table
        dense
        striped
        style={{ "--ledger-min-width": `${10 + (rows.length + 1) * 8}rem` } as CSSProperties}
        className="[--gutter:--spacing(4)] lg:[&_table]:w-full lg:[&_table]:min-w-(--ledger-min-width) lg:[&_table]:table-fixed"
      >
        <caption className="sr-only">{t("Gathering ledger")}</caption>
        <TableHead>
          <TableRow>
            {headers.map((column, index) => (
              <TableHeader
                key={column.key}
                scope="col"
                className={index > 0 ? "text-right lg:w-32" : undefined}
                aria-sort={sort.key === column.key ? sortDirection : undefined}
              >
                <button
                  type="button"
                  className="inline-flex items-center gap-1 rounded focus-visible:outline-2 focus-visible:outline-blue-500"
                  onClick={() => {
                    setSort({
                      key: column.key,
                      descending: sort.key === column.key ? !sort.descending : true,
                    });
                    setPage(0);
                  }}
                >
                  {column.name}
                  {sort.key === column.key ? (
                    <SortIcon aria-hidden="true" className="size-3" />
                  ) : null}
                </button>
              </TableHeader>
            ))}
          </TableRow>
        </TableHead>
        <TableBody>
          {pageDays.map((day) => (
            <TableRow key={day.date}>
              <TableCell>
                <time dateTime={day.date}>
                  {intl.dateTime(new Date(`${day.date}T00:00:00Z`), {
                    month: "short",
                    day: "numeric",
                    year: "numeric",
                    timeZone: "UTC",
                  })}
                </time>
              </TableCell>
              <TableCell
                className={`text-right tabular-nums ${day.reports === 0 ? "text-zinc-400 dark:text-zinc-600" : ""}`}
              >
                {intl.number(day.reports, { maximumFractionDigits: 0 })}
              </TableCell>
              {rows.map((row) => (
                <TableCell
                  key={row.key}
                  className={`text-right tabular-nums ${day.values[row.key] === 0 ? "text-zinc-400 dark:text-zinc-600" : ""}`}
                >
                  {intl.number(day.values[row.key], { maximumFractionDigits: 0 })}
                </TableCell>
              ))}
            </TableRow>
          ))}
          {visibleDays.length === 0 ? (
            <TableRow>
              <TableCell colSpan={headers.length}>{t("No reports in this date range.")}</TableCell>
            </TableRow>
          ) : null}
        </TableBody>
      </Table>
      <div className="flex items-center justify-between gap-3">
        <Text className="tabular-nums">
          {t("Page {page}/{pages}", { page: String(currentPage + 1), pages: String(pages) })}
        </Text>
        {pages > 1 ? (
          <div className="flex items-center gap-2">
            <Button plain disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}>
              {t("Previous")}
            </Button>
            <Button
              plain
              disabled={currentPage === pages - 1}
              onClick={() => setPage(currentPage + 1)}
            >
              {t("Next")}
            </Button>
          </div>
        ) : null}
      </div>
    </section>
  );
}
