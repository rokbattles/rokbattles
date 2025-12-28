import OlympianArenaTable from "@/components/olympian-arena/OlympianArenaTable";
import { Heading, Subheading } from "@/components/ui/Heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/Table";

export default function Page() {
  return (
    <>
      <Heading>Olympian Arena Reports</Heading>
      <div className="mt-8">
        <Subheading>Live feed (UTC)</Subheading>
      </div>
      <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-1/6">Time</TableHeader>
            <TableHeader>Sender</TableHeader>
            <TableHeader>Opponent</TableHeader>
            <TableHeader className="sm:w-1/6">Win Streak</TableHeader>
          </TableRow>
        </TableHead>
        <OlympianArenaTable />
      </Table>
    </>
  );
}
