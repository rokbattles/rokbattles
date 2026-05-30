"use client";

import { TableCell, TableRow } from "@/components/ui/table";

export default function ReportsErrorRow({ colSpan, error }: { colSpan: number; error: string }) {
  return (
    <TableRow>
      <TableCell colSpan={colSpan} role="status" aria-live="polite">
        {error}
      </TableCell>
    </TableRow>
  );
}
