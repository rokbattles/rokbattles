"use client";

import { cn } from "cn";
import { useExtracted } from "next-intl";
import type { CSSProperties } from "react";
import { TableCell } from "@/components/ui/table";

type ReportMapCellProps = {
  mapcode: string | null | undefined;
  banner: string | null | undefined;
};

export default function ReportMapCell({ mapcode: code, banner: image }: ReportMapCellProps) {
  const t = useExtracted();
  const mapcode = code || "Unknown";
  const banner = mapcode === "Unknown" ? null : image;

  return (
    <TableCell
      className={cn(
        "first:static w-1/8 tabular-nums",
        banner &&
          "before:content-[''] before:absolute before:inset-y-0 before:left-0 before:-z-10 before:w-[min(60%,24rem)] before:pointer-events-none before:bg-[image:var(--kvk-banner)] before:bg-left before:bg-cover before:bg-no-repeat before:opacity-30 dark:before:opacity-45 before:mask-[linear-gradient(to_right,black,transparent)]"
      )}
      style={banner ? ({ "--kvk-banner": `url("${banner}")` } as CSSProperties) : undefined}
    >
      {mapcode === "Unknown" ? t("Unknown") : mapcode === "Home" ? t("Home") : mapcode}
    </TableCell>
  );
}
