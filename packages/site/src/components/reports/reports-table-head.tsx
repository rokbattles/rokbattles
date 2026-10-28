"use client";

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
  return (
    <TableHead>
      <TableRow>
        {firstColumn === "map" ? (
          <TableHeader className="sm:min-w-32">Map</TableHeader>
        ) : (
          <TableHeader className="w-16">#</TableHeader>
        )}
        <TableHeader className="text-right sm:w-[17.5%] sm:min-w-24">Sender</TableHeader>
        <TableHeader className="text-center sm:min-w-24">Trade %</TableHeader>
        <TableHeader className="sm:w-[22.5%] sm:min-w-36">Opponent</TableHeader>
        <TableHeader className="hidden text-right sm:table-cell sm:min-w-28">
          Kill Count
        </TableHeader>
        <TableHeader className="hidden text-right sm:table-cell sm:min-w-28">
          {resultLabel}
        </TableHeader>
        <TableHeader className="text-right sm:min-w-28">When</TableHeader>
      </TableRow>
    </TableHead>
  );
}
