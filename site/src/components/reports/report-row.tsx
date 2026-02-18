"use client";

import { usePathname, useSearchParams } from "next/navigation";
import { TableCell, TableRow } from "@/components/ui/table";
import type { UseReportsResult } from "@/hooks/use-reports";
import { formatDurationShort, formatUtcDateTime } from "@/lib/datetime";
import ParticipantCell from "./participant-cell";

type Report = UseReportsResult["data"][number];

const numberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
});

function formatKillCount(value: number): string {
  if (!Number.isFinite(value)) {
    return "+0";
  }

  const normalized = Math.round(value);
  const sign = normalized >= 0 ? "+" : "";
  return `${sign}${numberFormatter.format(normalized)}`;
}

function formatTradePercent(value: number): string {
  if (!Number.isFinite(value)) {
    return "0%";
  }

  return `${Math.round(value)}%`;
}

export default function ReportRow({ report }: { report: Report }) {
  const searchParams = useSearchParams();
  const pathname = usePathname();
  const query = new URLSearchParams(searchParams.toString());
  const from = pathname === "/account/reports" ? "account-reports" : "reports";
  query.set("from", from);
  const queryString = query.toString();
  const encodedMailId = encodeURIComponent(report.mailId);
  const href = queryString ? `/report/${encodedMailId}?${queryString}` : `/report/${encodedMailId}`;

  return (
    <TableRow href={href}>
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
      <TableCell>{formatKillCount(report.killCount)}</TableCell>
      <TableCell>{formatTradePercent(report.tradePercent)}</TableCell>
      <TableCell>{formatDurationShort(report.timeStart, report.timeEnd)}</TableCell>
    </TableRow>
  );
}
