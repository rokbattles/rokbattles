"use client";

import { ArrowDown, ArrowUp, Download } from "lucide-react";

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

const dateFormatter = new Intl.DateTimeFormat("en-US", {
  month: "short",
  day: "numeric",
  year: "numeric",
  timeZone: "UTC",
});
const integerFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });

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
  const [requestedSort, setSort] = useState<LedgerSort>({ key: "date", descending: true });
  const [page, setPage] = useState(0);
  const [onlyReported, setOnlyReported] = useState(false);

  const headers: { key: LedgerSortKey; name: string }[] = [
    { key: "date", name: "Day (UTC)" },
    { key: "reports", name: "Reports" },
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
          <Subheading>Gathering ledger</Subheading>
          <Text className="mt-1">
            Exact totals for selected resources. Report counts include all resources.
          </Text>
        </div>
        <Button outline onClick={exportCsv} disabled={visibleDays.length === 0}>
          <Download data-slot="icon" />
          Export CSV
        </Button>
      </div>
      <label className="flex w-fit items-center gap-2 text-sm text-zinc-400">
        <input
          type="checkbox"
          checked={onlyReported}
          className="size-4 accent-zinc-700"
          onChange={(event) => {
            setOnlyReported(event.target.checked);
            setPage(0);
          }}
        />
        Only days with reports
      </label>
      <Table
        dense
        style={{ "--ledger-min-width": `${10 + (rows.length + 1) * 8}rem` } as CSSProperties}
        className="[--gutter:--spacing(4)] lg:[&_table]:w-full lg:[&_table]:min-w-(--ledger-min-width) lg:[&_table]:table-fixed"
      >
        <caption className="sr-only">Gathering ledger</caption>
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
                  {dateFormatter.format(new Date(`${day.date}T00:00:00Z`))}
                </time>
              </TableCell>
              <TableCell
                className={`text-right tabular-nums ${day.reports === 0 ? "text-zinc-600" : ""}`}
              >
                {integerFormatter.format(day.reports)}
              </TableCell>
              {rows.map((row) => (
                <TableCell
                  key={row.key}
                  className={`text-right tabular-nums ${day.values[row.key] === 0 ? "text-zinc-600" : ""}`}
                >
                  {integerFormatter.format(day.values[row.key])}
                </TableCell>
              ))}
            </TableRow>
          ))}
          {visibleDays.length === 0 ? (
            <TableRow>
              <TableCell colSpan={headers.length}>No reports in this date range.</TableCell>
            </TableRow>
          ) : null}
        </TableBody>
      </Table>
      <div className="flex items-center justify-between gap-3">
        <Text className="tabular-nums">{`Page ${String(currentPage + 1)}/${String(pages)}`}</Text>
        {pages > 1 ? (
          <div className="flex items-center gap-2">
            <Button plain disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}>
              Previous
            </Button>
            <Button
              plain
              disabled={currentPage === pages - 1}
              onClick={() => setPage(currentPage + 1)}
            >
              Next
            </Button>
          </div>
        ) : null}
      </div>
    </section>
  );
}
