import { getTranslations } from "next-intl/server";
import { MyPairingsContent } from "@/components/my-pairings/MyPairingsContent";
import { Heading } from "@/components/ui/Heading";

export default async function Page() {
  const t = await getTranslations("account");
  return (
    <>
      <Heading>{t("titles.pairings")}</Heading>
      <MyPairingsContent />
    </>
  );
}
