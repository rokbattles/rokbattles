"use client";

import { cn } from "cn";
import { usePathname, useSearchParams } from "next/navigation";
import { useExtracted } from "next-intl";
import { Badge } from "@/components/ui/badge";
import { TableCell, TableRow } from "@/components/ui/table";
import { formatDurationShort } from "@/lib/datetime";
import type { ReportsListItem } from "@/lib/types/reports-list";
import ParticipantCell from "./participant-cell";
import ReportMapCell from "./report-map-cell";
import ReportTimeCell from "./report-time-cell";

type ReportsTableRowProps = {
  report: ReportsListItem;
  now: Date | null;
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

export default function ReportsTableRow({ report, now, onOpenOverview }: ReportsTableRowProps) {
  const t = useExtracted();
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
      title={t("View battle report")}
      className={cn("relative isolate", report.battles > 1 && "cursor-pointer")}
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
      <ReportMapCell mapcode={report.kvkMapcode} banner={report.kvkBanner} />
      <TableCell className="text-right">
        <ParticipantCell
          primaryAwakened={report.sender.primaryCommanderAwakened}
          primaryId={report.sender.primaryCommanderId}
          secondaryAwakened={report.sender.secondaryCommanderAwakened}
          secondaryId={report.sender.secondaryCommanderId}
        />
      </TableCell>
      <TableCell className="w-1/10 text-center tabular-nums">
        {Math.round(report.tradePercent)}%
      </TableCell>
      <TableCell>
        <ParticipantCell
          primaryAwakened={report.opponent.primaryCommanderAwakened}
          primaryId={report.opponent.primaryCommanderId}
          secondaryAwakened={report.opponent.secondaryCommanderAwakened}
          secondaryId={report.opponent.secondaryCommanderId}
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
    </TableRow>
  );
}
