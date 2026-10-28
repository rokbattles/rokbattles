"use client";

import { TableCell, TableRow } from "@/components/ui/table";

export default function ReportsEmptyStateRow({ colSpan }: { colSpan: number }) {
  return (
    <TableRow>
      <TableCell colSpan={colSpan} role="status" aria-live="polite">
        No reports found.
      </TableCell>
    </TableRow>
  );
}
