"use client";

import { useExtracted } from "next-intl";
import type { ReactElement } from "react";
import { TableHead, TableHeader, TableRow } from "@/components/ui/table";

type ReportsTableHeadProps = {
  resultLabel: string;
  firstColumn?: "map" | "number";
};

export default function ReportsTableHead({
  resultLabel,
  firstColumn = "map",
}: ReportsTableHeadProps): ReactElement {
  const t = useExtracted();

  return (
    <TableHead>
      <TableRow>
        {firstColumn === "map" ? (
          <TableHeader className="sm:min-w-32">{t("Map")}</TableHeader>
        ) : (
          <TableHeader className="w-16">#</TableHeader>
        )}
        <TableHeader className="text-right sm:w-[17.5%] sm:min-w-24">{t("Sender")}</TableHeader>
        <TableHeader className="text-center sm:min-w-24">{t("Trade %")}</TableHeader>
        <TableHeader className="sm:w-[22.5%] sm:min-w-36">{t("Opponent")}</TableHeader>
        <TableHeader className="hidden text-right sm:table-cell sm:min-w-28">
          {t("Kill Count")}
        </TableHeader>
        <TableHeader className="hidden text-right sm:table-cell sm:min-w-28">
          {resultLabel}
        </TableHeader>
        <TableHeader className="text-right sm:min-w-28">{t("When")}</TableHeader>
      </TableRow>
    </TableHead>
  );
}
