"use client";

import { useExtracted } from "next-intl";
import ReportsTableHead from "@/components/reports/reports-table-head";
import { Button } from "@/components/ui/button";
import { Pagination } from "@/components/ui/pagination";
import { Table, TableBody } from "@/components/ui/table";
import { useOlympianArenaDuels } from "@/hooks/use-olympian-arena-duels";
import ReportsEmptyStateRow from "../reports/reports-empty-state-row";
import ReportsErrorRow from "../reports/reports-error-row";
import ReportsSkeletonRows from "../reports/reports-skeleton-rows";
import OlympianArenaRow from "./olympian-arena-row";

type OlympianArenaTableProps = {
  skeletonCount?: number;
};

export default function OlympianArenaTable({ skeletonCount = 10 }: OlympianArenaTableProps = {}) {
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
  } = useOlympianArenaDuels();

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
        <ReportsTableHead resultLabel={t("Win Count")} />
        <TableBody>
          {data.map((duel) => (
            <OlympianArenaRow key={duel.duelId} duel={duel} now={loadedAt} />
          ))}
          {loading && data.length === 0 ? <ReportsSkeletonRows count={skeletonCount} /> : null}
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
    </>
  );
}
