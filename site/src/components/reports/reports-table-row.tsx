"use client";

import { usePathname, useSearchParams } from "next/navigation";
import { TableCell, TableRow } from "@/components/ui/table";
import { formatDurationShort, formatUtcDateTime } from "@/lib/datetime";
import type { ReportsListItem } from "@/lib/types/reports-list";
import ParticipantCell from "./participant-cell";

type ReportsTableRowProps = {
  report: ReportsListItem;
  onOpenOverview: (report: ReportsListItem) => void;
};

function isPreviewableViewport() {
  if (typeof window === "undefined") {
    return false;
  }

  return !(
    window.matchMedia("(max-width: 767px)").matches ||
    window.matchMedia("(hover: none) and (pointer: coarse)").matches
  );
}

export default function ReportsTableRow({ report, onOpenOverview }: ReportsTableRowProps) {
  const searchParams = useSearchParams();
  const pathname = usePathname();
  const query = new URLSearchParams(searchParams.toString());
  const from = pathname === "/account/reports" ? "account-reports" : "reports";
  query.set("from", from);

  const queryString = query.toString();
  const encodedMailId = encodeURIComponent(report.mailId);
  const href = queryString ? `/report/${encodedMailId}?${queryString}` : `/report/${encodedMailId}`;

  return (
    <TableRow
      href={href}
      className={report.battles > 1 ? "cursor-pointer" : undefined}
      onClickCapture={(event) => {
        if (report.battles <= 1 || !isPreviewableViewport()) {
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
        onOpenOverview(report);
      }}
    >
      <TableCell className="font-medium text-zinc-950 dark:text-white">
        {formatUtcDateTime(report.timeStart)}
      </TableCell>
      <TableCell>
        <ParticipantCell
          primaryId={report.sender.primaryCommanderId}
          secondaryId={report.sender.secondaryCommanderId}
        />
      </TableCell>
      <TableCell>
        <ParticipantCell
          primaryId={report.opponent.primaryCommanderId}
          secondaryId={report.opponent.secondaryCommanderId}
        />
      </TableCell>
      <TableCell>{report.battles.toLocaleString()}</TableCell>
      <TableCell>+{Math.max(0, Math.round(report.killCount)).toLocaleString()}</TableCell>
      <TableCell>{Math.round(report.tradePercent)}%</TableCell>
      <TableCell>{formatDurationShort(report.timeStart, report.timeEnd)}</TableCell>
    </TableRow>
  );
}
