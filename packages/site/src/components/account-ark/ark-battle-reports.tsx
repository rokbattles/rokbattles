"use client";

import { type ReactElement, useEffect, useState } from "react";
import {
  ArkPagination,
  ArkRequestState,
  ArkSectionHeading,
  useArkNumber,
} from "@/components/account-ark/ark-shared";
import ReportsTableCells from "@/components/reports/reports-table-cells";
import ReportsTableHead from "@/components/reports/reports-table-head";
import { Table, TableBody, TableCell, TableRow } from "@/components/ui/table";
import { Text } from "@/components/ui/text";
import { useArkQuery } from "@/hooks/use-ark-query";
import type { ArkMatchDetail, ArkReportsResponse } from "@/lib/types/ark";

type ArkBattleReportsProps = {
  governorId: number;
  detail: ArkMatchDetail;
};

export function ArkBattleReports({ governorId, detail }: ArkBattleReportsProps): ReactElement {
  const [now, setNow] = useState<Date | null>(null);
  useEffect(() => {
    setNow(new Date());
  }, []);
  const number = useArkNumber();

  const [page, setPage] = useState(1);

  const available = detail.battleReports.status === "available";
  const query = useArkQuery<ArkReportsResponse>(
    available
      ? `/proxy/v1/governor/${governorId}/ark/${encodeURIComponent(detail.matchId)}/reports?page=${page}`
      : null
  );
  const coverage = query.data?.coverage ?? detail.battleReports;

  let content: ReactElement;

  if (query.loading) {
    content = <ArkRequestState>Loading battle reports…</ArkRequestState>;
  } else if (query.error) {
    content = (
      <ArkRequestState error retry={query.retry}>
        Failed to load battle reports.
      </ArkRequestState>
    );
  } else if (coverage.status === "ambiguous") {
    content = (
      <Text>
        More than one possible battle session was found. Reports are not linked until this match can
        be identified reliably.
      </Text>
    );
  } else if (!query.data?.items.length) {
    content = (
      <Text>
        No uploaded battle reports could be linked to this match. Upload the governor’s Ark battle
        mails to include them here.
      </Text>
    );
  } else {
    const firstReportNumber = (query.data.page - 1) * query.data.pageSize + 1;

    content = (
      <>
        <Table className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
          <ReportsTableHead resultLabel="Duration" firstColumn="number" />
          <TableBody>
            {query.data.items.map((report, index) => (
              <TableRow
                key={report.mailId}
                href={`/report/${encodeURIComponent(report.mailId)}?from=ark&matchId=${encodeURIComponent(detail.matchId)}`}
                title="View battle report"
              >
                <TableCell className="w-16 tabular-nums text-zinc-400">
                  {number(firstReportNumber + index)}
                </TableCell>
                <ReportsTableCells report={report} now={now} />
              </TableRow>
            ))}
          </TableBody>
        </Table>
        <ArkPagination
          page={page}
          pages={Math.max(1, Math.ceil(coverage.total / query.data.pageSize))}
          onChange={setPage}
        />
      </>
    );
  }

  return (
    <section className="space-y-4">
      <ArkSectionHeading
        title="Battle reports"
        description="All of your uploaded battle reports from this match."
      />
      {content}
    </section>
  );
}
