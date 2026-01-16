"use client";

import { TableCell, TableRow } from "@/components/ui/table";

export default function ErrorRow({
  colSpan,
  error,
}: {
  colSpan: number;
  error: string;
}) {
  return (
    <TableRow>
      <TableCell aria-live="polite" colSpan={colSpan} role="status">
        {error}
      </TableCell>
    </TableRow>
  );
}
