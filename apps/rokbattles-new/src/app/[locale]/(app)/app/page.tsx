import { BattleReportsTableBody } from "@/components/BattleReportsTableBody";
import { Button } from "@/components/ui/Button";
import { Heading, Subheading } from "@/components/ui/Heading";
import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/Table";

export default function Page() {
  return (
    <>
      <Heading>Explore Battle Reports</Heading>
      <div className="mt-8 flex items-end justify-between">
        <Subheading>Recent Battle Reports</Subheading>
        <Button>Filters</Button>
      </div>
      <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader>Battle Time</TableHeader>
            <TableHeader>Self Participant</TableHeader>
            <TableHeader>Enemy Participant</TableHeader>
            <TableHeader>Battles</TableHeader>
            <TableHeader>Battle Duration</TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          <BattleReportsTableBody />
        </TableBody>
      </Table>
    </>
  );
}
