import { ReportsDateToolbar } from "@/components/reports/reports-date-toolbar";
import { ReportsFilterBar } from "@/components/reports/reports-filter-bar";
import ReportsTable from "@/components/reports/reports-table";
import { Heading } from "@/components/ui/heading";

export default async function Page() {
  return (
    <>
      <Heading>Battle Reports</Heading>
      <div className="mt-4">
        <ReportsDateToolbar />
      </div>
      <ReportsFilterBar />
      <ReportsTable />
    </>
  );
}
