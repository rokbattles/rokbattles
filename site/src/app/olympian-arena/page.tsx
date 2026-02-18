import { getExtracted } from "next-intl/server";
import OlympianArenaTable from "@/components/olympian-arena/olympian-arena-table";
import { Heading, Subheading } from "@/components/ui/heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/table";

export default async function Page() {
  const t = await getExtracted();
  return (
    <>
      <Heading>{t("Olympian Arena Reports")}</Heading>
      <div className="mt-8">
        <Subheading>{t("Live feed (UTC)")}</Subheading>
      </div>
      <Table dense grid className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-36">{t("Time")}</TableHeader>
            <TableHeader>{t("Sender")}</TableHeader>
            <TableHeader>{t("Opponent")}</TableHeader>
            <TableHeader className="sm:w-32">{t("Kill Count")}</TableHeader>
            <TableHeader className="sm:w-32">{t("Trade %")}</TableHeader>
            <TableHeader className="sm:w-32">{t("Win Streak")}</TableHeader>
          </TableRow>
        </TableHead>
        <OlympianArenaTable />
      </Table>
    </>
  );
}
