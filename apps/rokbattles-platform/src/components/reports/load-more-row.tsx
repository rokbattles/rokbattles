"use client";

import { useTranslations } from "next-intl";
import { forwardRef } from "react";
import { Button } from "@/components/ui/button";
import { TableCell, TableRow } from "@/components/ui/table";

interface Props {
  colSpan: number;
  loading: boolean;
  onLoadMore: () => void;
}

const LoadMoreRow = forwardRef<HTMLDivElement, Props>(function LoadMoreRow(
  { colSpan, loading, onLoadMore },
  ref
) {
  const t = useTranslations("common");
  return (
    <TableRow>
      <TableCell colSpan={colSpan}>
        <div aria-hidden="true" className="h-1" ref={ref} />
        {loading ? (
          <div
            aria-live="polite"
            className="mb-1 text-center text-zinc-500 dark:text-zinc-400"
            role="status"
          >
            {t("states.loadingMore")}
          </div>
        ) : (
          <div className="flex justify-center">
            <Button
              className="text-sm/6"
              onClick={onLoadMore}
              plain
              type="button"
            >
              {t("actions.loadMore")}
            </Button>
          </div>
        )}
      </TableCell>
    </TableRow>
  );
});

export default LoadMoreRow;
