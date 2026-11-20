import { Funnel } from "lucide-react";

import { ReportsFilterDialog } from "@/components/reports/reports-filter-dialog";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";

export default async function Page() {
  return (
    <>
      <Heading>Battle Reports</Heading>
      <div className="mt-8 flex items-end justify-between">
        <Subheading>Live feed (UTC)</Subheading>
        <ReportsFilterDialog>
          <Funnel data-slot="icon" aria-hidden="true" />
          Filter
        </ReportsFilterDialog>
      </div>
      <ReportsTable />
    </>
  );
}
