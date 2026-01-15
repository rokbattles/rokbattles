"use client";

import ParticipantCell from "@/components/reports/participant-cell";
import { TableCell, TableRow, TableRowHeader } from "@/components/ui/Table";
import type { OlympianArenaDuelSummary } from "@/hooks/use-olympian-arena-duels";
import { formatUtcDateTime } from "@/lib/datetime";

function normalizeCommanderId(id: number | null): number {
  return typeof id === "number" && Number.isFinite(id) ? id : 0;
}

export default function OlympianArenaRow({ duel }: { duel: OlympianArenaDuelSummary }) {
  return (
    <TableRow href={`/olympian-arena/${duel.duelId}`}>
      <TableRowHeader className="font-medium text-zinc-950 dark:text-white">
        {formatUtcDateTime(duel.emailTime)}
      </TableRowHeader>
      <TableCell>
        <ParticipantCell
          primaryId={normalizeCommanderId(duel.entry.sender.primaryCommanderId)}
          secondaryId={normalizeCommanderId(duel.entry.sender.secondaryCommanderId)}
        />
      </TableCell>
      <TableCell>
        <ParticipantCell
          primaryId={normalizeCommanderId(duel.entry.opponent.primaryCommanderId)}
          secondaryId={normalizeCommanderId(duel.entry.opponent.secondaryCommanderId)}
        />
      </TableCell>
      <TableCell>{duel.winStreak.toLocaleString()}</TableCell>
    </TableRow>
  );
}
