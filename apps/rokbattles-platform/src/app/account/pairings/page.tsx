import { getTranslations } from "next-intl/server";
import { MyPairingsContent } from "@/components/my-pairings/my-pairings-content";
import { Heading } from "@/components/ui/Heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();
  const t = await getTranslations("account");
  return (
    <>
      <Heading>{t("titles.pairings")}</Heading>
      <MyPairingsContent />
    </>
  );
}
