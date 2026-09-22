"use client";

import { useExtracted } from "next-intl";
import { TableCell } from "@/components/ui/table";
import { formatElapsedShort, formatUtcDateTime, normalizeTimestampMillis } from "@/lib/datetime";

export default function ReportTimeCell({ time, now }: { time: number; now: Date | null }) {
  const t = useExtracted();
  const timestamp = normalizeTimestampMillis(time);
  const elapsed = formatElapsedShort(time, now);

  return (
    <TableCell className="w-1/8 text-right tabular-nums text-zinc-500 dark:text-zinc-400">
      <time
        dateTime={timestamp == null ? undefined : new Date(timestamp).toISOString()}
        title={timestamp == null ? formatUtcDateTime(time) : new Date(timestamp).toUTCString()}
      >
        {elapsed == null ? t("Unknown") : t("{elapsed} ago", { elapsed })}
      </time>
    </TableCell>
  );
}
