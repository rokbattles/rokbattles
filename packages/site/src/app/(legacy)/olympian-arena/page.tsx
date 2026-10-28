import OlympianArenaTable from "@/components/olympian-arena/olympian-arena-table";
import { Heading, Subheading } from "@/components/ui/heading";

export default async function Page() {
  return (
    <>
      <Heading>Olympian Arena Reports</Heading>
      <div className="mt-8">
        <Subheading>Live feed (UTC)</Subheading>
      </div>
      <OlympianArenaTable />
    </>
  );
}
