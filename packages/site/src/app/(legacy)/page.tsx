import { ReportsFilterBar } from "@/components/reports/reports-filter-bar";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";

export default async function Page() {
  return (
    <>
      <Heading>Battle Reports</Heading>
      <div className="mt-8">
        <Subheading>Live feed (UTC)</Subheading>
      </div>
      <ReportsFilterBar />
      <ReportsTable />
    </>
  );
}
