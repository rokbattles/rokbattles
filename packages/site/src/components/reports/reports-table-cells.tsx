"use client";

import { useExtracted } from "next-intl";
import type { ReactElement } from "react";
import { Badge } from "@/components/ui/badge";
import { TableCell } from "@/components/ui/table";
import { formatDurationShort } from "@/lib/datetime";
import type { ReportsListItem } from "@/lib/types/reports-list";
import ParticipantCell from "./participant-cell";
import ReportTimeCell from "./report-time-cell";

type ReportsTableCellsProps = {
  report: ReportsListItem;
  now: Date | null;
};

export default function ReportsTableCells({ report, now }: ReportsTableCellsProps): ReactElement {
  const t = useExtracted();

  return (
    <>
      <TableCell className="text-right">
        <ParticipantCell
          primaryAwakened={report.sender.primaryCommanderAwakened}
          primaryId={report.sender.primaryCommanderId}
          primarySkinId={report.sender.primaryCommanderSkins?.["22"]}
          secondaryAwakened={report.sender.secondaryCommanderAwakened}
          secondaryId={report.sender.secondaryCommanderId}
          secondarySkinId={report.sender.secondaryCommanderSkins?.["22"]}
        />
      </TableCell>
      <TableCell className="w-1/10 text-center tabular-nums">
        {Math.round(report.tradePercent)}%
      </TableCell>
      <TableCell>
        <ParticipantCell
          primaryAwakened={report.opponent.primaryCommanderAwakened}
          primaryId={report.opponent.primaryCommanderId}
          primarySkinId={report.opponent.primaryCommanderSkins?.["22"]}
          secondaryAwakened={report.opponent.secondaryCommanderAwakened}
          secondaryId={report.opponent.secondaryCommanderId}
          secondarySkinId={report.opponent.secondaryCommanderSkins?.["22"]}
        />
        {report.battles > 1 ? (
          <Badge className="ml-1 align-middle tabular-nums" title={t("Battles")}>
            +{(report.battles - 1).toLocaleString()}
          </Badge>
        ) : null}
      </TableCell>
      <TableCell className="hidden w-1/8 text-right tabular-nums sm:table-cell">
        +{Math.max(0, Math.round(report.killCount)).toLocaleString()}
      </TableCell>
      <TableCell className="hidden w-1/8 text-right tabular-nums sm:table-cell">
        {formatDurationShort(report.timeStart, report.timeEnd)}
      </TableCell>
      <ReportTimeCell time={report.timeStart} now={now} />
    </>
  );
}
