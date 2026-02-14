"use client";

import { usePathname, useSearchParams } from "next/navigation";
import { TableCell, TableRow } from "@/components/ui/table";
import type { UseReportsResult } from "@/hooks/use-reports";
import { formatDurationShort, formatUtcDateTime } from "@/lib/datetime";
import ParticipantCell from "./participant-cell";

type Report = UseReportsResult["data"][number];

export default function ReportRow({ report }: { report: Report }) {
  const searchParams = useSearchParams();
  const pathname = usePathname();
  const query = new URLSearchParams(searchParams.toString());
  const from = pathname === "/account/reports" ? "account-reports" : "reports";
  query.set("from", from);
  const queryString = query.toString();
  const href = queryString
    ? `/report/${report.parentHash}?${queryString}`
    : `/report/${report.parentHash}`;

  return (
    <TableRow key={report.parentHash} href={href}>
      <TableCell className="font-medium text-zinc-950 dark:text-white">
        {formatUtcDateTime(report.entry.startDate)}
      </TableCell>
      <TableCell>
        <ParticipantCell
          primaryId={report.entry.selfCommanderId}
          secondaryId={report.entry.selfSecondaryCommanderId}
        />
      </TableCell>
      <TableCell>
        <ParticipantCell
          primaryId={report.entry.enemyCommanderId}
          secondaryId={report.entry.enemySecondaryCommanderId}
        />
      </TableCell>
      <TableCell>{report.count.toLocaleString()}</TableCell>
      <TableCell>
        {formatDurationShort(report.timespan.firstStart, report.timespan.lastEnd)}
      </TableCell>
    </TableRow>
  );
}
