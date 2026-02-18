"use client";

import { useExtracted } from "next-intl";
import { TableCell, TableRow } from "@/components/ui/table";

export default function EmptyStateRow({ colSpan }: { colSpan: number }) {
  const t = useExtracted();
  return (
    <TableRow>
      <TableCell colSpan={colSpan} role="status" aria-live="polite">
        {t("No reports found.")}
      </TableCell>
    </TableRow>
  );
}
