"use client";

import { useExtracted } from "next-intl";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Pagination } from "@/components/ui/pagination";
import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { type ReportsScope, useReportsPage } from "@/hooks/use-reports-page";
import type { ReportsListItem } from "@/lib/types/reports-list";
import ReportsEmptyStateRow from "./reports-empty-state-row";
import ReportsErrorRow from "./reports-error-row";
import ReportsOverviewDrawer from "./reports-overview-drawer";
import ReportsSkeletonRows from "./reports-skeleton-rows";
import ReportsTableRow from "./reports-table-row";

const SkeletonWidths = ["w-24", "w-16", "w-16", "w-24", "w-20", "w-20", "w-24"] as const;
const HiddenMobileColumns = new Set([4, 5]);

type ReportsTableProps = {
  scope?: ReportsScope;
  skeletonCount?: number;
};

export default function ReportsTable({
  scope = "all",
  skeletonCount = 10,
}: ReportsTableProps = {}) {
  const t = useExtracted();
  const {
    data,
    loadedAt,
    loading,
    error,
    nextAfter,
    previousBefore,
    loadNextPage,
    loadPreviousPage,
  } = useReportsPage(scope);
  const [overviewRow, setOverviewRow] = useState<ReportsListItem | null>(null);

  const handleNextPage = async () => {
    await loadNextPage();
    window.scrollTo({ top: 0, behavior: "smooth" });
  };

  const handlePreviousPage = async () => {
    await loadPreviousPage();
    window.scrollTo({ top: 0, behavior: "smooth" });
  };

  return (
    <>
      <Table className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader>{t("Map")}</TableHeader>
            <TableHeader className="text-right">{t("Sender")}</TableHeader>
            <TableHeader className="text-center">{t("Trade %")}</TableHeader>
            <TableHeader>{t("Opponent")}</TableHeader>
            <TableHeader className="hidden text-right sm:table-cell">{t("Kill Count")}</TableHeader>
            <TableHeader className="hidden text-right sm:table-cell">{t("Duration")}</TableHeader>
            <TableHeader className="text-right">{t("When")}</TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          {data.map((report) => (
            <ReportsTableRow
              key={report.mailId}
              report={report}
              now={loadedAt}
              onOpenOverview={setOverviewRow}
            />
          ))}
          {loading && data.length === 0 ? (
            <ReportsSkeletonRows
              count={skeletonCount}
              widths={SkeletonWidths}
              hiddenOnMobile={HiddenMobileColumns}
            />
          ) : null}
          {!loading && !error && data.length === 0 ? <ReportsEmptyStateRow colSpan={7} /> : null}
          {error ? <ReportsErrorRow colSpan={7} error={error} /> : null}
        </TableBody>
      </Table>
      <Pagination className="mt-4">
        <span className="grow basis-0">
          <Button
            plain
            type="button"
            onClick={() => void handlePreviousPage()}
            disabled={!previousBefore || loading}
            aria-label={t("Previous page")}
            autoComplete="off"
          >
            {t("Previous")}
          </Button>
        </span>
        <span className="flex grow basis-0 justify-end">
          <Button
            plain
            type="button"
            onClick={() => void handleNextPage()}
            disabled={!nextAfter || loading}
            aria-label={t("Next page")}
            autoComplete="off"
          >
            {t("Next")}
          </Button>
        </span>
      </Pagination>
      <ReportsOverviewDrawer report={overviewRow} onClose={() => setOverviewRow(null)} />
    </>
  );
}
