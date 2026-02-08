"use client";
import { useExtracted } from "next-intl";
import { useState } from "react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { formatDurationShort, formatUtcDateTime } from "@/lib/datetime";
import { formatTradePercentage } from "@/lib/trade";
import type {
  ExploreBattleReportRow,
  ExploreBattleReportsPage,
} from "@/lib/types/explore-battle-reports";
import CommanderPair from "./commander-pair";
import ExploreReportOverviewDrawer from "./explore-report-overview-drawer";

export default function ExploreBattleReportsTable({
  reports,
}: {
  reports: ExploreBattleReportsPage;
}) {
  const t = useExtracted();
  const [overviewRow, setOverviewRow] = useState<ExploreBattleReportRow | null>(
    null
  );
  const isMobileViewport = () =>
    typeof window !== "undefined" &&
    (window.matchMedia("(max-width: 767px)").matches ||
      window.matchMedia("(hover: none) and (pointer: coarse)").matches);

  return (
    <>
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
            <TableHeader className="w-32">{t("Kill Count")}</TableHeader>
            <TableHeader className="w-28">{t("Trade %")}</TableHeader>
            <TableHeader className="w-28">{t("Battles")}</TableHeader>
            <TableHeader className="w-28">{t("Duration")}</TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          {reports.rows.map((row) => {
            const killCount =
              (row.summary.opponent.dead ?? 0) +
              (row.summary.opponent.severelyWounded ?? 0);

            return (
              <TableRow
                className={row.battles > 1 ? "cursor-pointer" : undefined}
                href={`/report/${encodeURIComponent(row.mailId)}`}
                key={row.id}
                onClickCapture={(event) => {
                  if (row.battles <= 1 || isMobileViewport()) {
                    return;
                  }

                  if (
                    event.button !== 0 ||
                    event.metaKey ||
                    event.ctrlKey ||
                    event.altKey ||
                    event.shiftKey
                  ) {
                    return;
                  }

                  event.preventDefault();
                  setOverviewRow(row);
                }}
              >
                <TableCell className="tabular-nums">
                  {formatUtcDateTime(row.startTimestamp)}
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
                  +{killCount.toLocaleString()}
                </TableCell>
                <TableCell className="tabular-nums">
                  {formatTradePercentage(row.tradePercentage)}
                </TableCell>
                <TableCell className="tabular-nums">
                  {row.battles.toLocaleString()}
                </TableCell>
                <TableCell className="tabular-nums">
                  {formatDurationShort(row.startTimestamp, row.endTimestamp)}
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
      <ExploreReportOverviewDrawer
        onClose={() => setOverviewRow(null)}
        report={overviewRow}
      />
    </>
  );
}
