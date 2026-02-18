import { FunnelIcon } from "@heroicons/react/16/solid";
import { getExtracted } from "next-intl/server";
import { ReportsFilterDialog } from "@/components/reports/reports-filter-dialog";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/table";

export default async function Page() {
  const t = await getExtracted();

  return (
    <>
      <Heading>{t("Battle Reports")}</Heading>
      <div className="mt-8 flex items-end justify-between">
        <Subheading>{t("Live feed (UTC)")}</Subheading>
        <ReportsFilterDialog>
          <FunnelIcon />
          {t("Filter")}
        </ReportsFilterDialog>
      </div>
      <Table dense grid className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-36">{t("Time")}</TableHeader>
            <TableHeader>{t("Sender")}</TableHeader>
            <TableHeader>{t("Opponent")}</TableHeader>
            <TableHeader className="sm:w-32">{t("Battles")}</TableHeader>
            <TableHeader className="sm:w-32">{t("Kill Count")}</TableHeader>
            <TableHeader className="sm:w-32">{t("Trade %")}</TableHeader>
            <TableHeader className="sm:w-32">{t("Duration")}</TableHeader>
          </TableRow>
        </TableHead>
        <ReportsTable />
      </Table>
    </>
  );
}
