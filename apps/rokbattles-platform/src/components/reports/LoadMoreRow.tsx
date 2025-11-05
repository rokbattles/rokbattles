"use client";

import { forwardRef } from "react";
import { TableCell, TableRow } from "@/components/ui/Table";

type Props = { colSpan: number; loading: boolean };

const LoadMoreRow = forwardRef<HTMLDivElement, Props>(function LoadMoreRow(
  { colSpan, loading },
  ref
) {
  return (
    <TableRow aria-hidden>
      <TableCell colSpan={colSpan}>
        <div ref={ref} className="h-1" />
        {loading ? (
          <div className="mb-1 text-center text-zinc-500 dark:text-zinc-400">
            Loading more&hellip;
          </div>
        ) : (
          <span className="sr-only">Load more</span>
        )}
      </TableCell>
    </TableRow>
  );
});

export default LoadMoreRow;
