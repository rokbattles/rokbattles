"use client";

import { TableCell, TableRow } from "@/components/ui/Table";
import { cn } from "@/lib/cn";

type Props = {
  count?: number;
  widths?: readonly string[];
};

const DefaultWidths = ["w-24", "w-36", "w-36", "w-16", "w-24"] as const;

export default function SkeletonRows({ count = 10, widths = DefaultWidths }: Props) {
  return (
    <>
      <TableRow>
        <TableCell colSpan={widths.length} className="sr-only" role="status" aria-live="polite">
          Loading reports...
        </TableCell>
      </TableRow>
      {Array.from({ length: count }, (_, r) => (
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
      ))}
    </>
  );
}
