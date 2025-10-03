"use client";

import { TableCell, TableRow } from "@/components/ui/Table";
import { useBattleReports } from "@/hooks/useBattleReports";
import { formatDurationShort, formatUtcDateTime } from "@/lib/datetime";

export function BattleReportsTableBody() {
  const { reports, loading } = useBattleReports();

  if (loading) {
    return (
      <TableRow>
        <TableCell colSpan={5}>Loading battle reports&hellip;</TableCell>
      </TableRow>
    );
  }

  if (reports.length === 0) {
    return (
      <TableRow>
        <TableCell colSpan={5}>No battle reports available.</TableCell>
      </TableRow>
    );
  }

  return (
    <>
      {reports.map((report) => {
        const { entry, timespan } = report;
        const hasSelfSecondary = Boolean(entry.selfSecondaryCommanderId);
        const hasEnemySecondary = Boolean(entry.enemySecondaryCommanderId);

        return (
          <TableRow key={report.parentHash} href={`/app/report/${report.parentHash}`}>
            <TableCell>{formatUtcDateTime(entry.startDate)}</TableCell>
            <TableCell>
              <div className="flex flex-col">
                <span>{entry.selfCommanderId}</span>
                {hasSelfSecondary ? (
                  <span className="text-zinc-600 dark:text-zinc-400">
                    {entry.selfSecondaryCommanderId}
                  </span>
                ) : null}
              </div>
            </TableCell>
            <TableCell>
              <div className="flex flex-col">
                <span>{entry.enemyCommanderId}</span>
                {hasEnemySecondary ? (
                  <span className="text-zinc-600 dark:text-zinc-400">
                    {entry.enemySecondaryCommanderId}
                  </span>
                ) : null}
              </div>
            </TableCell>
            <TableCell>{report.count}</TableCell>
            <TableCell>{formatDurationShort(timespan.firstStart, timespan.lastEnd)}</TableCell>
          </TableRow>
        );
      })}
    </>
  );
}
