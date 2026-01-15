"use client";

import { useCallback, useRef } from "react";
import { TableBody } from "@/components/ui/Table";
import { useFavoriteReports } from "@/hooks/use-favorite-reports";
import { useInfiniteScroll } from "@/hooks/use-infinite-scroll";
import EmptyStateRow from "./empty-state-row";
import ErrorRow from "./error-row";
import LoadMoreRow from "./load-more-row";
import ReportRow from "./report-row";
import SkeletonRows from "./skeleton-rows";

type FavoriteReportsTableProps = {
  skeletonCount?: number;
};

export default function FavoriteReportsTable({
  skeletonCount = 10,
}: FavoriteReportsTableProps = {}) {
  const { data, loading, error, cursor, loadMore } = useFavoriteReports();
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
        <ReportRow key={report.parentHash} report={report} />
      ))}
      {loading && data.length === 0 ? <SkeletonRows count={skeletonCount} /> : null}
      {!loading && !error && data.length === 0 ? <EmptyStateRow colSpan={5} /> : null}
      {error ? <ErrorRow colSpan={5} error={error} /> : null}
      {cursor ? (
        <LoadMoreRow
          colSpan={5}
          loading={loading}
          onLoadMore={handleLoadMore}
          ref={setSentinelRef}
        />
      ) : null}
    </TableBody>
  );
}
