"use client";

import { useExtracted } from "next-intl";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { formatUtcDateTime } from "@/lib/datetime";
import { formatTradePercentage } from "@/lib/trade";
import type { ExploreOlympianArenaPage } from "@/lib/types/explore-olympian-arena";
import CommanderPair from "./commander-pair";

export default function ExploreOlympianArenaTable({
  reports,
}: {
  reports: ExploreOlympianArenaPage;
}) {
  const t = useExtracted();

  return (
    <Table
      className="mt-2 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]"
      dense
      grid
    >
      <TableHead>
        <TableRow>
          <TableHeader className="w-36">{t("Time")}</TableHeader>
          <TableHeader>{t("Sender")}</TableHeader>
          <TableHeader>{t("Opponent")}</TableHeader>
          <TableHeader className="w-32">{t("Trade %")}</TableHeader>
          <TableHeader className="w-32">{t("Win Streak")}</TableHeader>
        </TableRow>
      </TableHead>
      <TableBody>
        {reports.rows.map((row) => (
          <TableRow key={row.id}>
            <TableCell className="tabular-nums">
              {formatUtcDateTime(row.mailTime)}
            </TableCell>
            <TableCell>
              <CommanderPair
                primary={row.senderCommanders.primary}
                secondary={row.senderCommanders.secondary}
              />
            </TableCell>
            <TableCell>
              <CommanderPair
                primary={row.opponentCommanders.primary}
                secondary={row.opponentCommanders.secondary}
              />
            </TableCell>
            <TableCell className="tabular-nums">
              {formatTradePercentage(row.tradePercentage)}
            </TableCell>
            <TableCell className="tabular-nums">
              {row.winStreak.toLocaleString()}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
