import { getTranslations } from "next-intl/server";
import OlympianArenaTable from "@/components/olympian-arena/olympian-arena-table";
import { Heading, Subheading } from "@/components/ui/heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/table";

export default async function Page() {
  const [t, tCommon] = await Promise.all([getTranslations("duels"), getTranslations("common")]);
  return (
    <>
      <Heading>{t("listTitle")}</Heading>
      <div className="mt-8">
        <Subheading>{tCommon("headings.liveFeed")}</Subheading>
      </div>
      <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-1/6">{tCommon("labels.time")}</TableHeader>
            <TableHeader>{tCommon("labels.sender")}</TableHeader>
            <TableHeader>{tCommon("labels.opponent")}</TableHeader>
            <TableHeader className="sm:w-1/6">{t("table.winStreak")}</TableHeader>
          </TableRow>
        </TableHead>
        <OlympianArenaTable />
      </Table>
    </>
  );
}
