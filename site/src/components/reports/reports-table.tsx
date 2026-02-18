"use client";

import { useCallback, useRef } from "react";
import { TableBody } from "@/components/ui/table";
import { useInfiniteScroll } from "@/hooks/use-infinite-scroll";
import { type ReportsScope, useReports } from "@/hooks/use-reports";
import EmptyStateRow from "./empty-state-row";
import ErrorRow from "./error-row";
import LoadMoreRow from "./load-more-row";
import ReportRow from "./report-row";
import SkeletonRows from "./skeleton-rows";

const SkeletonWidths = ["w-24", "w-36", "w-36", "w-16", "w-20", "w-20", "w-24"] as const;

type ReportsTableProps = {
  scope?: ReportsScope;
  skeletonCount?: number;
};

export default function ReportsTable({
  scope = "all",
  skeletonCount = 10,
}: ReportsTableProps = {}) {
  const { data, loading, error, cursor, loadMore } = useReports({ scope });
  const loadingRef = useRef(false);

  const handleLoadMore = useCallback(async () => {
    if (loadingRef.current || loading || !cursor) {
      return;
    }

    loadingRef.current = true;
    try {
      await loadMore();
    } finally {
      loadingRef.current = false;
    }
  }, [loading, cursor, loadMore]);

  const setSentinelRef = useInfiniteScroll({
    enabled: Boolean(cursor),
    loading,
    onLoadMore: handleLoadMore,
    rootMargin: "256px 0px 0px 0px",
    threshold: 0.01,
  });

  return (
    <TableBody>
      {data.map((report) => (
        <ReportRow key={report.mailId} report={report} />
      ))}
      {loading && data.length === 0 ? (
        <SkeletonRows count={skeletonCount} widths={SkeletonWidths} />
      ) : null}
      {!loading && !error && data.length === 0 ? <EmptyStateRow colSpan={7} /> : null}
      {error ? <ErrorRow colSpan={7} error={error} /> : null}
      {cursor ? (
        <LoadMoreRow
          colSpan={7}
          loading={loading}
          onLoadMore={handleLoadMore}
          ref={setSentinelRef}
        />
      ) : null}
    </TableBody>
  );
}
