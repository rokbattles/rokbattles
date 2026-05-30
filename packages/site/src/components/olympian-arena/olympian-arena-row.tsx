"use client";

import ParticipantCell from "@/components/reports/participant-cell";
import { TableCell, TableRow } from "@/components/ui/table";
import type { OlympianArenaDuelSummary } from "@/hooks/use-olympian-arena-duels";
import { formatUtcDateTime } from "@/lib/datetime";

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

export default function OlympianArenaRow({ duel }: { duel: OlympianArenaDuelSummary }) {
  return (
    <TableRow href={`/olympian-arena/${duel.duelId}`}>
      <TableCell className="font-medium text-zinc-950 dark:text-white">
        {formatUtcDateTime(duel.mailTime)}
      </TableCell>
      <TableCell>
        <ParticipantCell
          primaryAwakened={duel.entry.sender.primaryCommanderAwakened}
          primaryId={duel.entry.sender.primaryCommanderId}
          secondaryAwakened={duel.entry.sender.secondaryCommanderAwakened}
          secondaryId={duel.entry.sender.secondaryCommanderId}
        />
      </TableCell>
      <TableCell>
        <ParticipantCell
          primaryAwakened={duel.entry.opponent.primaryCommanderAwakened}
          primaryId={duel.entry.opponent.primaryCommanderId}
          secondaryAwakened={duel.entry.opponent.secondaryCommanderAwakened}
          secondaryId={duel.entry.opponent.secondaryCommanderId}
        />
      </TableCell>
      <TableCell>{formatKillCount(duel.killCount)}</TableCell>
      <TableCell>{formatTradePercent(duel.tradePercent)}</TableCell>
      <TableCell>{duel.winStreak.toLocaleString()}</TableCell>
    </TableRow>
  );
}
