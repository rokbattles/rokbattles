import { FunnelIcon } from "@heroicons/react/16/solid";
import { getTranslations } from "next-intl/server";
import { ReportsFilterDialog } from "@/components/reports/reports-filter-dialog";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/table";

export default async function Page() {
  const [t, tCommon] = await Promise.all([getTranslations("reports"), getTranslations("common")]);
  return (
    <>
      <Heading>{t("title")}</Heading>
      <div className="mt-8 flex items-end justify-between">
        <Subheading>{tCommon("headings.liveFeed")}</Subheading>
        <ReportsFilterDialog>
          <FunnelIcon />
          {t("filter.trigger")}
        </ReportsFilterDialog>
      </div>
      <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-1/6">{tCommon("labels.time")}</TableHeader>
            <TableHeader>{tCommon("labels.sender")}</TableHeader>
            <TableHeader>{tCommon("labels.opponent")}</TableHeader>
            <TableHeader className="sm:w-1/6">{tCommon("labels.battles")}</TableHeader>
            <TableHeader className="sm:w-1/6">{t("table.duration")}</TableHeader>
          </TableRow>
        </TableHead>
        <ReportsTable />
      </Table>
    </>
  );
}
