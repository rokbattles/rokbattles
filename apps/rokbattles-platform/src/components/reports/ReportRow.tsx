"use client";

import { TableCell, TableRow } from "@/components/ui/Table";
import type { UseReportsResult } from "@/hooks/useReports";
import { formatDurationShort, formatUtcDateTime } from "@/lib/datetime";
import ParticipantCell from "./ParticipantCell";

type Report = UseReportsResult["data"][number];

export default function ReportRow({ report }: { report: Report }) {
  return (
    <TableRow key={report.parentHash} href={`/report/${report.parentHash}`}>
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
