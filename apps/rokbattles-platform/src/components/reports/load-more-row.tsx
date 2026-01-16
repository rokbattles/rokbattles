"use client";

import { useTranslations } from "next-intl";
import { forwardRef } from "react";
import { Button } from "@/components/ui/button";
import { TableCell, TableRow } from "@/components/ui/table";

type Props = { colSpan: number; loading: boolean; onLoadMore: () => void };

const LoadMoreRow = forwardRef<HTMLDivElement, Props>(function LoadMoreRow(
  { colSpan, loading, onLoadMore },
  ref
) {
  const t = useTranslations("common");
  return (
    <TableRow>
      <TableCell colSpan={colSpan}>
        <div ref={ref} className="h-1" aria-hidden="true" />
        {loading ? (
          <div
            className="mb-1 text-center text-zinc-500 dark:text-zinc-400"
            role="status"
            aria-live="polite"
          >
            {t("states.loadingMore")}
          </div>
        ) : (
          <div className="flex justify-center">
            <Button plain type="button" onClick={onLoadMore} className="text-sm/6">
              {t("actions.loadMore")}
            </Button>
          </div>
        )}
      </TableCell>
    </TableRow>
  );
});

export default LoadMoreRow;
