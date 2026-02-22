import { Badge } from "@/components/ui/badge";
import { TableCell, TableRow } from "@/components/ui/table";
import {
  formatArkAllianceLabel,
  getArkIsetAlliance,
  getArkSethAlliance,
  getArkWinnerSide,
} from "@/lib/ark/format";
import { formatUtcDateTime } from "@/lib/datetime";
import { formatWholeNumber } from "@/lib/loot/format";
import type { ArkMatchRecord } from "@/lib/types/ark";

export type ArkMatchHistoryRowLabels = {
  unknownAlliance: string;
  unknownWinner: string;
  seth: string;
  iset: string;
  rowLinkTitle: string;
};

type ArkMatchHistoryRowProps = {
  row: ArkMatchRecord;
  labels: ArkMatchHistoryRowLabels;
};

export function ArkMatchHistoryRow({ row, labels }: ArkMatchHistoryRowProps) {
  const detailsHref = `/account/ark/${encodeURIComponent(row.matchId)}`;
  const sethAlliance = getArkSethAlliance(row);
  const isetAlliance = getArkIsetAlliance(row);
  const winnerSide = getArkWinnerSide(row);
  const winnerLabel =
    winnerSide === "seth"
      ? labels.seth
      : winnerSide === "iset"
        ? labels.iset
        : labels.unknownWinner;
  const winnerBadgeColor = winnerSide === "seth" ? "red" : winnerSide === "iset" ? "blue" : "zinc";
  const membersLabel = `${formatWholeNumber(sethAlliance?.members ?? 0)} vs ${formatWholeNumber(isetAlliance?.members ?? 0)}`;
  const scoreLabel = `${formatWholeNumber(sethAlliance?.score ?? 0)} vs ${formatWholeNumber(isetAlliance?.score ?? 0)}`;

  return (
    <TableRow href={detailsHref} title={labels.rowLinkTitle}>
      <TableCell className="tabular-nums">{formatUtcDateTime(row.mailTimeMillis)}</TableCell>
      <TableCell>{formatArkAllianceLabel(sethAlliance, labels.unknownAlliance)}</TableCell>
      <TableCell>{formatArkAllianceLabel(isetAlliance, labels.unknownAlliance)}</TableCell>
      <TableCell>
        <Badge color={winnerBadgeColor}>{winnerLabel}</Badge>
      </TableCell>
      <TableCell className="tabular-nums">{membersLabel}</TableCell>
      <TableCell className="tabular-nums">{scoreLabel}</TableCell>
    </TableRow>
  );
}
