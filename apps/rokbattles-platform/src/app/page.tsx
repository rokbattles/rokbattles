import { FunnelIcon } from "@heroicons/react/16/solid";
import { ReportsFilterDialog } from "@/components/reports/ReportsFilterDialog";
import ReportsTable from "@/components/reports/ReportsTable";
import { Heading, Subheading } from "@/components/ui/Heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/Table";

export default function Page() {
  return (
    <>
      <Heading>Battle Reports</Heading>
      <div className="mt-8 flex items-end justify-between">
        <Subheading>Live feed (UTC)</Subheading>
        <ReportsFilterDialog>
          <FunnelIcon />
          Filter
        </ReportsFilterDialog>
      </div>
      <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-1/6">Time</TableHeader>
            <TableHeader>Self Participant</TableHeader>
            <TableHeader>Enemy Participant</TableHeader>
            <TableHeader className="sm:w-1/6">Battles</TableHeader>
            <TableHeader className="sm:w-1/6">Duration</TableHeader>
          </TableRow>
        </TableHead>
        <ReportsTable />
      </Table>
    </>
  );
}
