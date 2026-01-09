import { getTranslations } from "next-intl/server";
import { MyRewardsContent } from "@/components/my-rewards/MyRewardsContent";
import { Heading } from "@/components/ui/Heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();
  const t = await getTranslations("account");

  return (
    <>
      <Heading>{t("titles.rewards")}</Heading>
      <MyRewardsContent />
    </>
  );
}
