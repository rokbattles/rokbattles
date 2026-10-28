"use client";

import type { ReactElement } from "react";
import { ArkAllianceEmblem } from "@/components/account-ark/ark-alliance-emblem";
import { useArkLeagueLabels, useArkNumber } from "@/components/account-ark/ark-shared";
import { Badge } from "@/components/ui/badge";
import { TableCell, TableRow } from "@/components/ui/table";
import { getArkOutcome, getArkTeams } from "@/lib/ark/analytics";
import { formatArkAllianceLabel } from "@/lib/ark/format";
import type { ArkMatchRecord } from "@/lib/types/ark";

const dateFormatter = new Intl.DateTimeFormat("en-US", {
  year: "numeric",
  month: "short",
  day: "numeric",
  timeZone: "UTC",
});
const timeFormatter = new Intl.DateTimeFormat("en-US", {
  hour: "2-digit",
  minute: "2-digit",
  hourCycle: "h23",
  timeZone: "UTC",
});

type ArkMatchHistoryRowProps = { row: ArkMatchRecord };

export function ArkMatchHistoryRow({ row }: ArkMatchHistoryRowProps): ReactElement {
  const number = useArkNumber();
  const leagues = useArkLeagueLabels();

  const { own, opponent } = getArkTeams(row);
  const outcome = getArkOutcome(row);
  const outcomes = { win: "Win", loss: "Loss", draw: "Draw", unknown: "Unknown" };
  const colors = { win: "green", loss: "red", draw: "amber", unknown: "zinc" } as const;

  return (
    <TableRow href={`/account/ark/${encodeURIComponent(row.matchId)}`} title="View Ark recap">
      <TableCell className="tabular-nums">
        <div>{dateFormatter.format(row.mailTimeMillis)}</div>
        <div className="mt-1 text-xs text-zinc-400">
          {timeFormatter.format(row.mailTimeMillis)} UTC
        </div>
      </TableCell>
      <TableCell>
        <div className="flex items-center gap-3">
          <ArkAllianceEmblem logo={own?.logo ?? null} isBlue={own?.isBlue ?? null} />
          <div>
            <div className="font-medium">{formatArkAllianceLabel(own, "Unknown alliance")}</div>
            <div className="mt-1 text-xs text-zinc-400">
              {`vs ${formatArkAllianceLabel(opponent, "Unknown alliance")}`}
            </div>
          </div>
        </div>
      </TableCell>
      <TableCell className="text-zinc-400">{leagues[row.league]}</TableCell>
      <TableCell>
        <Badge color={colors[outcome]}>{outcomes[outcome]}</Badge>
      </TableCell>
      <TableCell className="text-right tabular-nums">
        <span className="font-medium">{number(own?.score)}</span>
        <span className="text-zinc-400">{` / ${number(opponent?.score)}`}</span>
      </TableCell>
      <TableCell className="text-right tabular-nums">{number(row.personalScore)}</TableCell>
    </TableRow>
  );
}
