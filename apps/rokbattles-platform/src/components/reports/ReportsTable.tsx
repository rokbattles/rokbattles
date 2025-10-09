"use client";

import { useEffect, useMemo, useRef } from "react";
import { TableBody, TableCell, TableRow } from "@/components/ui/Table";
import { useCommanderName } from "@/hooks/useCommanderName";
import { useReports } from "@/hooks/useReports";
import { cn } from "@/lib/cn";
import { formatDurationShort, formatUtcDateTime } from "@/lib/datetime";

const skeletonWidths = ["w-24", "w-36", "w-36", "w-16", "w-24"] as const;

function ParticipantCell({ primaryId, secondaryId }: { primaryId: number; secondaryId: number }) {
  const primaryName = useCommanderName(primaryId);
  const secondaryName = useCommanderName(secondaryId);

  const primaryFallback =
    typeof primaryId === "number" && Number.isFinite(primaryId) && primaryId > 0
      ? String(primaryId)
      : undefined;
  const secondaryFallback =
    typeof secondaryId === "number" && Number.isFinite(secondaryId) && secondaryId > 0
      ? String(secondaryId)
      : undefined;

  const primaryLabel = primaryName ?? primaryFallback;
  const secondaryLabel = secondaryName ?? secondaryFallback;

  const hasPrimary = Boolean(primaryLabel);
  const hasSecondary = Boolean(secondaryLabel);

  if (!hasPrimary && !hasSecondary) {
    return <span className="text-zinc-500 dark:text-zinc-400">Unknown</span>;
  }

  return (
    <div className="flex flex-col gap-1">
      {hasPrimary ? <span>{primaryLabel}</span> : null}
      {hasSecondary ? (
        <span className="text-zinc-600 dark:text-zinc-400">{secondaryLabel}</span>
      ) : null}
    </div>
  );
}

export default function ReportsTable() {
  const { data, loading, error, cursor, loadMore } = useReports();
  const sentinelRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const node = sentinelRef.current;
    if (!node || !cursor) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            loadMore();
            break;
          }
        }
      },
      {
        rootMargin: "256px 0px 0px 0px",
        threshold: 0.01,
      }
    );

    observer.observe(node);

    return () => observer.disconnect();
  }, [cursor, loadMore]);

  const skeletonRows = useMemo(() => {
    return Array.from({ length: 10 }, (_, index) => (
      // biome-ignore lint/suspicious/noArrayIndexKey: it's safe
      <TableRow key={index} aria-hidden>
        {skeletonWidths.map((width, cellIndex) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: it's safe
          <TableCell key={cellIndex}>
            <div
              className={cn("h-4 animate-pulse rounded bg-zinc-200/80 dark:bg-zinc-700/60", width)}
            />
          </TableCell>
        ))}
      </TableRow>
    ));
  }, []);

  return (
    <TableBody>
      {data.map((report) => (
        <TableRow key={report.parentHash}>
          <TableCell className="font-medium text-zinc-950 dark:text-white">
            {formatUtcDateTime(report.entry.startDate)}
          </TableCell>
          <TableCell>
            <ParticipantCell
              primaryId={report.entry.selfCommanderId}
              secondaryId={report.entry.selfSecondaryCommanderId}
            />
          </TableCell>
          <TableCell>
            <ParticipantCell
              primaryId={report.entry.enemyCommanderId}
              secondaryId={report.entry.enemySecondaryCommanderId}
            />
          </TableCell>
          <TableCell>{report.count.toLocaleString()}</TableCell>
          <TableCell>
            {formatDurationShort(report.timespan.firstStart, report.timespan.lastEnd)}
          </TableCell>
        </TableRow>
      ))}

      {loading && data.length === 0 ? skeletonRows : null}

      {!loading && !error && data.length === 0 ? (
        <TableRow>
          <TableCell
            colSpan={5}
            className="py-8 text-center text-sm text-zinc-500 dark:text-zinc-400"
          >
            No reports found.
          </TableCell>
        </TableRow>
      ) : null}

      {error ? (
        <TableRow>
          <TableCell colSpan={5} className="py-4 text-sm text-rose-500 dark:text-rose-400">
            Failed to load reports: {error}
          </TableCell>
        </TableRow>
      ) : null}

      {cursor ? (
        <TableRow aria-hidden>
          <TableCell colSpan={5}>
            <div ref={sentinelRef} className="h-1" />
            {loading ? (
              <div className="mb-1 text-center text-zinc-500 dark:text-zinc-400">
                Loading more&hellip;
              </div>
            ) : (
              <span className="sr-only">Load more</span>
            )}
          </TableCell>
        </TableRow>
      ) : null}
    </TableBody>
  );
}
