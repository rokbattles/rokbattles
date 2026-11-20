import { Funnel } from "lucide-react";

import { ReportsFilterDialog } from "@/components/reports/reports-filter-dialog";
import { ReportsModeTabs } from "@/components/reports/reports-mode-tabs";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";

export default async function Page() {
  return (
    <>
      <Heading>Battle Reports</Heading>
      <div className="mt-8 flex flex-wrap items-end justify-between gap-4">
        <Subheading>Live feed (UTC)</Subheading>
        <ReportsModeTabs />
        <ReportsFilterDialog>
          <Funnel data-slot="icon" aria-hidden="true" />
          Filter
        </ReportsFilterDialog>
      </div>
      <ReportsTable />
    </>
  );
}
