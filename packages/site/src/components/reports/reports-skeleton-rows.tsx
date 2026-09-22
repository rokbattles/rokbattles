"use client";

import { cn } from "cn";
import { useExtracted } from "next-intl";
import { TableCell, TableRow } from "@/components/ui/table";

type Props = {
  count?: number;
};

const Columns = [
  { width: "w-24", cell: "w-1/8" },
  { width: "w-16", cell: "text-right" },
  { width: "w-16", cell: "w-1/10 text-center" },
  { width: "w-24", cell: "" },
  { width: "w-20", cell: "hidden w-1/8 text-right sm:table-cell" },
  { width: "w-20", cell: "hidden w-1/8 text-right sm:table-cell" },
  { width: "w-24", cell: "w-1/8 text-right" },
] as const;

export default function ReportsSkeletonRows({ count = 10 }: Props) {
  const t = useExtracted();
  return (
    <>
      <TableRow className="sr-only" role="status" aria-live="polite">
        <TableCell colSpan={Columns.length}>{t("Loading reports...")}</TableCell>
      </TableRow>
      {Array.from({ length: count }, (_, r) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: its okay
        <TableRow key={r} aria-hidden>
          {Columns.map(({ width, cell }, c) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: its okay
            <TableCell key={c} className={cell}>
              <div
                className={cn(
                  "inline-block h-4 max-w-full animate-pulse rounded align-middle bg-zinc-200/80 dark:bg-zinc-700/60",
                  width
                )}
              />
            </TableCell>
          ))}
        </TableRow>
      ))}
    </>
  );
}
