"use client";

import { useTranslations } from "next-intl";
import { useContext } from "react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useKvks } from "@/hooks/use-kvks";
import { GovernorContext } from "@/providers/governor-context";

const columnCount = 5;

export function MyKvksContent() {
  const t = useTranslations("kvks");
  const tCommon = useTranslations("common");
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("My KVKs must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;
  const { data, loading, error } = useKvks(activeGovernor?.governorId);
  const rows = data?.items ?? [];

  if (!activeGovernor) {
    return null;
  }

  return (
    <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
      <TableHead>
        <TableRow>
          <TableHeader className="w-10">#</TableHeader>
          <TableHeader>{t("table.kvk")}</TableHeader>
          <TableHeader className="sm:w-1/6">{t("table.stateDate")}</TableHeader>
          <TableHeader className="sm:w-1/6">{t("table.endDate")}</TableHeader>
          <TableHeader className="sm:w-1/6">{t("table.battleReports")}</TableHeader>
        </TableRow>
      </TableHead>
      <TableBody>
        {loading ? (
          <TableRow>
            <TableCell colSpan={columnCount} role="status" aria-live="polite">
              {t("states.loading")}
            </TableCell>
          </TableRow>
        ) : error ? (
          <TableRow>
            <TableCell colSpan={columnCount} role="status" aria-live="polite">
              {error}
            </TableCell>
          </TableRow>
        ) : rows.length === 0 ? (
          <TableRow>
            <TableCell colSpan={columnCount} role="status" aria-live="polite">
              {t("states.empty")}
            </TableCell>
          </TableRow>
        ) : (
          rows.map((row, index) => (
            <TableRow key={row.serverId}>
              <TableCell className="tabular-nums text-zinc-500 dark:text-zinc-400">
                {index + 1}
              </TableCell>
              <TableCell className="tabular-nums">{row.serverId}</TableCell>
              <TableCell>{tCommon("labels.na")}</TableCell>
              <TableCell>{tCommon("labels.na")}</TableCell>
              <TableCell className="tabular-nums">{row.reportCount.toLocaleString()}</TableCell>
            </TableRow>
          ))
        )}
      </TableBody>
    </Table>
  );
}
