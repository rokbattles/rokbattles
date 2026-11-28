import { Funnel } from "lucide-react";

import { ReportsFilterDialog } from "@/components/reports/reports-filter-dialog";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

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
