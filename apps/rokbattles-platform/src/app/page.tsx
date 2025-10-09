import { FunnelIcon } from "@heroicons/react/16/solid";
import ReportsTable from "@/components/battle/ReportsTable";
import { Button } from "@/components/ui/Button";
import { Heading, Subheading } from "@/components/ui/Heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/Table";

export default function Page() {
  return (
    <>
      <Heading>Battle Reports</Heading>
      <div className="mt-8 flex items-end justify-between">
        <Subheading>Most recent</Subheading>
        <Button>
          <FunnelIcon />
          Filters
        </Button>
      </div>
      <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader>Time</TableHeader>
            <TableHeader>Self Participant</TableHeader>
            <TableHeader>Enemy Participant</TableHeader>
            <TableHeader>Battles</TableHeader>
            <TableHeader>Duration</TableHeader>
          </TableRow>
        </TableHead>
        <ReportsTable />
      </Table>
    </>
  );
}
