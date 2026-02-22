import { getExtracted } from "next-intl/server";
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

type ArkMatchHistoryRowProps = {
  row: ArkMatchRecord;
};

export async function ArkMatchHistoryRow({ row }: ArkMatchHistoryRowProps) {
  const t = await getExtracted();
  const detailsHref = `/account/ark/${encodeURIComponent(row.matchId)}`;
  const sethAlliance = getArkSethAlliance(row);
  const isetAlliance = getArkIsetAlliance(row);
  const winnerSide = getArkWinnerSide(row);
  const winnerLabel =
    winnerSide === "seth" ? t("Seth") : winnerSide === "iset" ? t("Iset") : t("Unknown");
  const winnerBadgeColor = winnerSide === "seth" ? "red" : winnerSide === "iset" ? "blue" : "zinc";
  const membersLabel = `${formatWholeNumber(isetAlliance?.members ?? 0)} vs ${formatWholeNumber(sethAlliance?.members ?? 0)}`;
  const scoreLabel = `${formatWholeNumber(isetAlliance?.score ?? 0)} vs ${formatWholeNumber(sethAlliance?.score ?? 0)}`;

  return (
    <TableRow href={detailsHref} title={t("View Ark match details")}>
      <TableCell className="tabular-nums">{formatUtcDateTime(row.mailTimeMillis)}</TableCell>
      <TableCell>{formatArkAllianceLabel(isetAlliance, t("Unknown alliance"))}</TableCell>
      <TableCell>{formatArkAllianceLabel(sethAlliance, t("Unknown alliance"))}</TableCell>
      <TableCell>
        <Badge color={winnerBadgeColor}>{winnerLabel}</Badge>
      </TableCell>
      <TableCell className="tabular-nums">{membersLabel}</TableCell>
      <TableCell className="tabular-nums">{scoreLabel}</TableCell>
    </TableRow>
  );
}
