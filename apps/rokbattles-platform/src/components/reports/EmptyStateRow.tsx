"use client";

import { useTranslations } from "next-intl";
import { TableCell, TableRow } from "@/components/ui/Table";

export default function EmptyStateRow({ colSpan }: { colSpan: number }) {
  const t = useTranslations("reports");
  return (
    <TableRow>
      <TableCell colSpan={colSpan} role="status" aria-live="polite">
        {t("states.empty")}
      </TableCell>
    </TableRow>
  );
}
