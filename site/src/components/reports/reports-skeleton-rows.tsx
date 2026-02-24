"use client";

import { useExtracted } from "next-intl";
import { TableCell, TableRow } from "@/components/ui/table";
import { cn } from "@/lib/cn";

type Props = {
  count?: number;
  widths?: readonly string[];
};

const DefaultWidths = ["w-24", "w-36", "w-36", "w-16", "w-24"] as const;

export default function ReportsSkeletonRows({ count = 10, widths = DefaultWidths }: Props) {
  const t = useExtracted();
  return (
    <>
      <TableRow className="sr-only" role="status" aria-live="polite">
        <TableCell colSpan={widths.length}>{t("Loading reports...")}</TableCell>
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
