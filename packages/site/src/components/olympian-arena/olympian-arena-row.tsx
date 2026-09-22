"use client";

import { useExtracted } from "next-intl";
import ParticipantCell from "@/components/reports/participant-cell";
import ReportMapCell from "@/components/reports/report-map-cell";
import ReportTimeCell from "@/components/reports/report-time-cell";
import { Badge } from "@/components/ui/badge";
import { TableCell, TableRow } from "@/components/ui/table";
import type { OlympianArenaDuelSummary } from "@/hooks/use-olympian-arena-duels";

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

export default function OlympianArenaRow({
  duel,
  now,
}: {
  duel: OlympianArenaDuelSummary;
  now: Date | null;
}) {
  const t = useExtracted();

  return (
    <TableRow
      href={`/olympian-arena/${duel.duelId}`}
      title={t("View battle report")}
      className="relative isolate"
    >
      <ReportMapCell mapcode={duel.kvkMapcode} banner={duel.kvkBanner} />
      <TableCell className="text-right">
        <ParticipantCell
          primaryAwakened={duel.entry.sender.primaryCommanderAwakened}
          primaryId={duel.entry.sender.primaryCommanderId}
          secondaryAwakened={duel.entry.sender.secondaryCommanderAwakened}
          secondaryId={duel.entry.sender.secondaryCommanderId}
        />
      </TableCell>
      <TableCell className="w-1/10 text-center tabular-nums">
        {formatTradePercent(duel.tradePercent)}
      </TableCell>
      <TableCell>
        <ParticipantCell
          primaryAwakened={duel.entry.opponent.primaryCommanderAwakened}
          primaryId={duel.entry.opponent.primaryCommanderId}
          secondaryAwakened={duel.entry.opponent.secondaryCommanderAwakened}
          secondaryId={duel.entry.opponent.secondaryCommanderId}
        />
        {duel.battles > 1 ? (
          <Badge className="ml-1 align-middle tabular-nums" title={t("Battles")}>
            +{(duel.battles - 1).toLocaleString()}
          </Badge>
        ) : null}
      </TableCell>
      <TableCell className="hidden w-1/8 text-right tabular-nums sm:table-cell">
        {formatKillCount(duel.killCount)}
      </TableCell>
      <TableCell className="hidden w-1/8 text-right tabular-nums sm:table-cell">
        {duel.winStreak.toLocaleString()}
      </TableCell>
      <ReportTimeCell time={duel.mailTime} now={now} />
    </TableRow>
  );
}
