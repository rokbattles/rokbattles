"use client";

import { useTranslations } from "next-intl";
import { TableCell, TableRow } from "@/components/ui/table";
import { cn } from "@/lib/cn";

type Props = {
  count?: number;
  widths?: readonly string[];
};

const DefaultWidths = ["w-24", "w-36", "w-36", "w-16", "w-24"] as const;

export default function SkeletonRows({
  count = 10,
  widths = DefaultWidths,
}: Props) {
  const t = useTranslations("common");
  return (
    <>
      <TableRow>
        <TableCell
          aria-live="polite"
          className="sr-only"
          colSpan={widths.length}
          role="status"
        >
          {t("states.loading")}
        </TableCell>
      </TableRow>
      {Array.from({ length: count }, (_, r) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: its okay
        <TableRow aria-hidden key={r}>
          {widths.map((w, c) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: its okay
            <TableCell key={c}>
              <div
                className={cn(
                  "h-4 animate-pulse rounded bg-zinc-200/80 dark:bg-zinc-700/60",
                  w
                )}
              />
            </TableCell>
          ))}
        </TableRow>
      ))}
    </>
  );
}
