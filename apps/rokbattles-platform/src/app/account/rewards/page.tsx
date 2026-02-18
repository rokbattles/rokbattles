import { getExtracted } from "next-intl/server";
import { MyRewardsContent } from "@/components/my-rewards/my-rewards-content";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();
  const t = await getExtracted();

  return (
    <>
      <Heading>{t("My Rewards")}</Heading>
      <MyRewardsContent />
    </>
  );
}
