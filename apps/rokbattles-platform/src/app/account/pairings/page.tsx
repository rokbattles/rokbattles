import { getExtracted } from "next-intl/server";
import { MyPairingsContent } from "@/components/my-pairings/my-pairings-content";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();
  const t = await getExtracted();
  return (
    <>
      <Heading>{t("My Pairings")}</Heading>
      <MyPairingsContent />
    </>
  );
}
