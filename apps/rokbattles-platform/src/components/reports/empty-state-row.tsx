"use client";

import { useTranslations } from "next-intl";
import { TableCell, TableRow } from "@/components/ui/table";

export default function EmptyStateRow({ colSpan }: { colSpan: number }) {
  const t = useTranslations("reports");
  return (
    <TableRow>
      <TableCell aria-live="polite" colSpan={colSpan} role="status">
        {t("states.empty")}
      </TableCell>
    </TableRow>
  );
}
