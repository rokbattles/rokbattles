import { getTranslations } from "next-intl/server";
import { MyKvksContent } from "@/components/my-kvks/my-kvks-content";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();
  const t = await getTranslations("account");

  return (
    <>
      <Heading>{t("titles.kvks")}</Heading>
      <MyKvksContent />
    </>
  );
}
