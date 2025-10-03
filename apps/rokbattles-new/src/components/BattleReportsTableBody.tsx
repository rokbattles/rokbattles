"use client";

import { TableCell, TableRow } from "@/components/ui/Table";

export function BattleReportsTableBody() {
  return (
    <TableRow>
      <TableCell>UTC 10/02 14:45</TableCell>
      <TableCell>
        <div className="flex flex-col">
          <span>Gorgo</span>
          <span className="text-zinc-600 dark:text-zinc-400">Atilla</span>
        </div>
      </TableCell>
      <TableCell>
        <div className="flex flex-col">
          <span>Scipio</span>
          <span className="text-zinc-600 dark:text-zinc-400">Atilla</span>
        </div>
      </TableCell>
      <TableCell>10</TableCell>
      <TableCell>2m 43s</TableCell>
    </TableRow>
  );
}
