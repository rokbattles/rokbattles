"use client";

import { cn } from "cn";
import { usePathname, useSearchParams } from "next/navigation";
import { useExtracted } from "next-intl";
import { TableRow } from "@/components/ui/table";
import type { ReportsListItem } from "@/lib/types/reports-list";
import ReportMapCell from "./report-map-cell";
import ReportsTableCells from "./reports-table-cells";

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
      <ReportsTableCells report={report} now={now} />
    </TableRow>
  );
}
