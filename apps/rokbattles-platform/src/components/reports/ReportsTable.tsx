"use client";

import { TableBody } from "@/components/ui/Table";
import { useInfiniteScroll } from "@/hooks/useInfiniteScroll";
import { type ReportsScope, useReports } from "@/hooks/useReports";
import EmptyStateRow from "./EmptyStateRow";
import ErrorRow from "./ErrorRow";
import LoadMoreRow from "./LoadMoreRow";
import ReportRow from "./ReportRow";
import SkeletonRows from "./SkeletonRows";

type ReportsTableProps = {
  scope?: ReportsScope;
  skeletonCount?: number;
};

export default function ReportsTable({
  scope = "all",
  skeletonCount = 10,
}: ReportsTableProps = {}) {
  const { data, loading, error, cursor, loadMore } = useReports({ scope });

  const setSentinelRef = useInfiniteScroll({
    enabled: Boolean(cursor),
    loading,
    onLoadMore: loadMore,
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
      {cursor ? <LoadMoreRow colSpan={5} loading={loading} ref={setSentinelRef} /> : null}
    </TableBody>
  );
}
