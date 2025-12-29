"use client";

import { TableCell, TableRow } from "@/components/ui/Table";

export default function EmptyStateRow({ colSpan }: { colSpan: number }) {
  return (
    <TableRow>
      <TableCell colSpan={colSpan} role="status" aria-live="polite">
        No reports found.
      </TableCell>
    </TableRow>
  );
}
