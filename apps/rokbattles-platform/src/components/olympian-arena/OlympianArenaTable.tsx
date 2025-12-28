"use client";

import EmptyStateRow from "@/components/reports/EmptyStateRow";
import ErrorRow from "@/components/reports/ErrorRow";
import LoadMoreRow from "@/components/reports/LoadMoreRow";
import SkeletonRows from "@/components/reports/SkeletonRows";
import { TableBody } from "@/components/ui/Table";
import { useInfiniteScroll } from "@/hooks/useInfiniteScroll";
import type { UseOlympianArenaDuelsResult } from "@/hooks/useOlympianArenaDuels";
import { useOlympianArenaDuels } from "@/hooks/useOlympianArenaDuels";
import OlympianArenaRow from "./OlympianArenaRow";

const SkeletonWidths = ["w-24", "w-36", "w-36", "w-20"] as const;

type UseDuelsHook = () => UseOlympianArenaDuelsResult;

type OlympianArenaTableProps = {
  useDuelsHook?: UseDuelsHook;
  skeletonCount?: number;
};

export default function OlympianArenaTable({
  useDuelsHook = useOlympianArenaDuels,
  skeletonCount = 10,
}: OlympianArenaTableProps = {}) {
  const { data, loading, error, cursor, loadMore } = useDuelsHook();

  const setSentinelRef = useInfiniteScroll({
    enabled: Boolean(cursor),
    loading,
    onLoadMore: loadMore,
    rootMargin: "256px 0px 0px 0px",
    threshold: 0.01,
  });

  return (
    <TableBody>
      {data.map((duel) => (
        <OlympianArenaRow key={duel.duelId} duel={duel} />
      ))}
      {loading && data.length === 0 ? (
        <SkeletonRows count={skeletonCount} widths={SkeletonWidths} />
      ) : null}
      {!loading && !error && data.length === 0 ? <EmptyStateRow colSpan={4} /> : null}
      {error ? <ErrorRow colSpan={4} error={error} /> : null}
      {cursor ? <LoadMoreRow colSpan={4} loading={loading} ref={setSentinelRef} /> : null}
    </TableBody>
  );
}
