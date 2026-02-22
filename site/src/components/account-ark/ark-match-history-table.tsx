import { getExtracted } from "next-intl/server";
import { ArkMatchHistoryRow } from "@/components/account-ark/ark-match-history-row";
import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import type { ArkMatchRecord } from "@/lib/types/ark";

type ArkMatchHistoryTableProps = {
  rows: ArkMatchRecord[];
};

export async function ArkMatchHistoryTable({ rows }: ArkMatchHistoryTableProps) {
  const t = await getExtracted();

  return (
    <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
      <TableHead>
        <TableRow>
          <TableHeader className="sm:w-36">{t("Time")}</TableHeader>
          <TableHeader>{t("Iset")}</TableHeader>
          <TableHeader>{t("Seth")}</TableHeader>
          <TableHeader className="sm:w-24">{t("Winner")}</TableHeader>
          <TableHeader className="sm:w-24">{t("Members")}</TableHeader>
          <TableHeader className="sm:w-40">{t("Score")}</TableHeader>
        </TableRow>
      </TableHead>
      <TableBody>
        {rows.map((row) => (
          <ArkMatchHistoryRow key={row.matchId} row={row} />
        ))}
      </TableBody>
    </Table>
  );
}
