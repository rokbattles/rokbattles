"use client";

import { TableCell, TableRow } from "@/components/ui/Table";

export default function EmptyStateRow({ colSpan }: { colSpan: number }) {
  return (
    <TableRow>
      <TableCell colSpan={colSpan}>No reports found.</TableCell>
    </TableRow>
  );
}
