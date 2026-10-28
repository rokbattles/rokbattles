"use client";

import { cn } from "cn";

import type { CSSProperties } from "react";
import { TableCell } from "@/components/ui/table";

type ReportMapCellProps = {
  mapcode: string | null | undefined;
  banner: string | null | undefined;
};

export default function ReportMapCell({ mapcode: code, banner: image }: ReportMapCellProps) {
  const mapcode = code || "Unknown";
  const banner = mapcode === "Unknown" ? null : image;

  return (
    <TableCell
      className={cn(
        "isolate w-1/8 tabular-nums",
        banner &&
          "before:content-[''] before:absolute before:inset-y-0 before:left-0 before:-z-10 before:w-96 before:pointer-events-none before:bg-[image:var(--kvk-banner)] before:bg-left before:bg-cover before:bg-no-repeat before:opacity-45 before:mask-[linear-gradient(to_right,black,transparent)]"
      )}
      style={banner ? ({ "--kvk-banner": `url("${banner}")` } as CSSProperties) : undefined}
    >
      {mapcode === "Unknown" ? "Unknown" : mapcode === "Home" ? "Home" : mapcode}
    </TableCell>
  );
}
