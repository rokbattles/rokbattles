import { getExtracted } from "next-intl/server";
import OlympianArenaTable from "@/components/olympian-arena/olympian-arena-table";
import { Heading, Subheading } from "@/components/ui/heading";

export default async function Page() {
  const t = await getExtracted();
  return (
    <>
      <Heading>{t("Olympian Arena Reports")}</Heading>
      <div className="mt-8">
        <Subheading>{t("Live feed (UTC)")}</Subheading>
      </div>
      <OlympianArenaTable />
    </>
  );
}
