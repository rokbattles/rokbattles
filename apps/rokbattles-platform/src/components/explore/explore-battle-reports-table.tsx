"use client";

import {
  EllipsisVerticalIcon,
  EyeIcon,
  ShareIcon,
  StarIcon,
} from "@heroicons/react/16/solid";
import { useExtracted } from "next-intl";
import { useState } from "react";
import {
  Dropdown,
  DropdownButton,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/dropdown";
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
            <TableHeader className="w-32">{t("Trade %")}</TableHeader>
            <TableHeader className="w-32">{t("Battles")}</TableHeader>
            <TableHeader className="w-32">{t("Duration")}</TableHeader>
            <TableHeader className="w-16">
              <span className="sr-only">{t("Action")}</span>
            </TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          {reports.rows.map((row) => (
            <TableRow
              href={`/report/${encodeURIComponent(row.mailId)}`}
              key={row.id}
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
                {formatTradePercentage(row.tradePercentage)}
              </TableCell>
              <TableCell className="tabular-nums">
                {row.battles.toLocaleString()}
              </TableCell>
              <TableCell className="tabular-nums">
                {formatDurationShort(row.startTimestamp, row.endTimestamp)}
              </TableCell>
              <TableCell>
                <Dropdown>
                  <DropdownButton
                    className="relative z-10 min-w-0 px-2 py-1.5"
                    plain
                    type="button"
                  >
                    <EllipsisVerticalIcon data-slot="icon" />
                  </DropdownButton>
                  <DropdownMenu anchor="bottom end">
                    {row.battles > 1 ? (
                      <DropdownItem
                        onClick={(event) => {
                          event.preventDefault();
                          event.stopPropagation();
                          setOverviewRow(row);
                        }}
                      >
                        <EyeIcon data-slot="icon" />
                        <DropdownLabel>{t("Overview")}</DropdownLabel>
                      </DropdownItem>
                    ) : null}
                    <DropdownItem disabled>
                      <StarIcon data-slot="icon" />
                      <DropdownLabel>{t("Favorite")}</DropdownLabel>
                    </DropdownItem>
                    <DropdownItem disabled>
                      <ShareIcon data-slot="icon" />
                      <DropdownLabel>{t("Share")}</DropdownLabel>
                    </DropdownItem>
                  </DropdownMenu>
                </Dropdown>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <ExploreReportOverviewDrawer
        onClose={() => setOverviewRow(null)}
        report={overviewRow}
      />
    </>
  );
}
