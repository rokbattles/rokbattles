import { getTranslations } from "next-intl/server";
import FavoriteReportsTable from "@/components/reports/FavoriteReportsTable";
import { Heading } from "@/components/ui/Heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/Table";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUser();
  const tAccount = await getTranslations("account");
  const tReports = await getTranslations("reports");
  const tCommon = await getTranslations("common");
  return (
    <>
      <Heading>{tAccount("titles.favorites")}</Heading>
      <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-1/6">{tCommon("labels.time")}</TableHeader>
            <TableHeader>{tCommon("labels.sender")}</TableHeader>
            <TableHeader>{tCommon("labels.opponent")}</TableHeader>
            <TableHeader className="sm:w-1/6">{tCommon("labels.battles")}</TableHeader>
            <TableHeader className="sm:w-1/6">{tReports("table.duration")}</TableHeader>
          </TableRow>
        </TableHead>
        <FavoriteReportsTable />
      </Table>
    </>
  );
}
