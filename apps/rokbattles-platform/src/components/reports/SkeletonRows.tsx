"use client";

import { memo, useMemo } from "react";
import { TableCell, TableRow } from "@/components/ui/Table";
import { cn } from "@/lib/cn";

type Props = {
  count?: number;
  widths?: readonly string[];
};

const DefaultWidths = ["w-24", "w-36", "w-36", "w-16", "w-24"] as const;

const SkeletonRows = memo(function SkeletonRows({ count = 10, widths = DefaultWidths }: Props) {
  const rows = useMemo(() => {
    return Array.from({ length: count }, (_, r) => (
      // biome-ignore lint/suspicious/noArrayIndexKey: its okay
      <TableRow key={r} aria-hidden>
        {widths.map((w, c) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: its okay
          <TableCell key={c}>
            <div
              className={cn("h-4 animate-pulse rounded bg-zinc-200/80 dark:bg-zinc-700/60", w)}
            />
          </TableCell>
        ))}
      </TableRow>
    ));
  }, [count, widths]);

  return <>{rows}</>;
});

export default SkeletonRows;
