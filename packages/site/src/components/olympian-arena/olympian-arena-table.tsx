"use client";

import { useExtracted } from "next-intl";
import { Button } from "@/components/ui/button";
import { Pagination } from "@/components/ui/pagination";
import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { GameTranslate } from "@/components/v1/game-translate";
import { useOlympianArenaDuels } from "@/hooks/use-olympian-arena-duels";
import ReportsEmptyStateRow from "../reports/reports-empty-state-row";
import ReportsErrorRow from "../reports/reports-error-row";
import ReportsSkeletonRows from "../reports/reports-skeleton-rows";
import OlympianArenaRow from "./olympian-arena-row";

const SkeletonWidths = ["w-24", "w-36", "w-36", "w-20", "w-20", "w-20"] as const;

type OlympianArenaTableProps = {
  skeletonCount?: number;
};

export default function OlympianArenaTable({ skeletonCount = 10 }: OlympianArenaTableProps = {}) {
  const t = useExtracted();
  const { data, loading, error, nextAfter, previousBefore, loadNextPage, loadPreviousPage } =
    useOlympianArenaDuels();

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
      <Table dense grid className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-36">{t("Time")}</TableHeader>
            <TableHeader>{t("Sender")}</TableHeader>
            <TableHeader>{t("Opponent")}</TableHeader>
            <TableHeader className="sm:w-32">
              <GameTranslate value="LC_COMMON_KILLING_AMOUNT" />
            </TableHeader>
            <TableHeader className="sm:w-32">{t("Trade %")}</TableHeader>
            <TableHeader className="sm:w-32">{t("Win Streak")}</TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          {data.map((duel) => (
            <OlympianArenaRow key={duel.duelId} duel={duel} />
          ))}
          {loading && data.length === 0 ? (
            <ReportsSkeletonRows count={skeletonCount} widths={SkeletonWidths} />
          ) : null}
          {!loading && !error && data.length === 0 ? <ReportsEmptyStateRow colSpan={6} /> : null}
          {error ? <ReportsErrorRow colSpan={6} error={error} /> : null}
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
          >
            {t("Next")}
          </Button>
        </span>
      </Pagination>
    </>
  );
}
