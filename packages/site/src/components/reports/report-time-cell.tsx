"use client";

import { TableCell } from "@/components/ui/table";
import { formatElapsedShort, formatUtcDateTime, normalizeTimestampMillis } from "@/lib/datetime";

export default function ReportTimeCell({ time, now }: { time: number; now: Date | null }) {
  const timestamp = normalizeTimestampMillis(time);
  const elapsed = formatElapsedShort(time, now);

  return (
    <TableCell className="w-1/8 text-right tabular-nums text-zinc-400">
      <time
        dateTime={timestamp == null ? undefined : new Date(timestamp).toISOString()}
        title={timestamp == null ? formatUtcDateTime(time) : new Date(timestamp).toUTCString()}
      >
        {elapsed == null ? "Unknown" : `${elapsed} ago`}
      </time>
    </TableCell>
  );
}
