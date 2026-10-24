import { Funnel } from "lucide-react";
import { getExtracted } from "next-intl/server";
import { ReportsFilterDialog } from "@/components/reports/reports-filter-dialog";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";

export default async function Page() {
  const t = await getExtracted();

  return (
    <>
      <Heading>{t("Battle Reports")}</Heading>
      <div className="mt-8 flex items-end justify-between">
        <Subheading>{t("Live feed (UTC)")}</Subheading>
        <ReportsFilterDialog>
          <Funnel data-slot="icon" aria-hidden="true" />
          {t("Filter")}
        </ReportsFilterDialog>
      </div>
      <ReportsTable />
    </>
  );
}
