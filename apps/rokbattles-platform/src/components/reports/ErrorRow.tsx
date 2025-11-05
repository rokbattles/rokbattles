"use client";

import { TableCell, TableRow } from "@/components/ui/Table";

export default function ErrorRow({ colSpan, error }: { colSpan: number; error: string }) {
  return (
    <TableRow>
      <TableCell colSpan={colSpan}>Failed to load reports: {error}</TableCell>
    </TableRow>
  );
}
