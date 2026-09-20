"use client";

import { cn } from "cn";
import { usePathname, useSearchParams } from "next/navigation";
import { useExtracted } from "next-intl";
import type { CSSProperties } from "react";
import { Badge } from "@/components/ui/badge";
import { TableCell, TableRow } from "@/components/ui/table";
import {
  formatDurationShort,
  formatElapsedShort,
  formatUtcDateTime,
  normalizeTimestampMillis,
} from "@/lib/datetime";
import type { ReportsListItem } from "@/lib/types/reports-list";
import ParticipantCell from "./participant-cell";

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
  const mapcode = report.kvkMapcode || "Unknown";
  const banner = mapcode === "Unknown" ? null : report.kvkBanner;
  const timestamp = normalizeTimestampMillis(report.timeStart);
  const elapsed = formatElapsedShort(report.timeStart, now);

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
      <TableCell
        className={cn(
          "first:static w-1/8 tabular-nums",
          banner &&
            "before:content-[''] before:absolute before:inset-y-0 before:left-0 before:-z-10 before:w-[min(60%,24rem)] before:pointer-events-none before:bg-[image:var(--kvk-banner)] before:bg-left before:bg-cover before:bg-no-repeat before:opacity-30 dark:before:opacity-45 before:mask-[linear-gradient(to_right,black,transparent)]"
        )}
        style={banner ? ({ "--kvk-banner": `url("${banner}")` } as CSSProperties) : undefined}
      >
        {mapcode === "Unknown" ? t("Unknown") : mapcode === "Home" ? t("Home") : mapcode}
      </TableCell>
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
      <TableCell className="w-1/8 text-right tabular-nums text-zinc-500 dark:text-zinc-400">
        <time
          dateTime={timestamp == null ? undefined : new Date(timestamp).toISOString()}
          title={
            timestamp == null
              ? formatUtcDateTime(report.timeStart)
              : new Date(timestamp).toUTCString()
          }
        >
          {elapsed == null ? t("Unknown") : t("{elapsed} ago", { elapsed })}
        </time>
      </TableCell>
    </TableRow>
  );
}
