import { redirect } from "next/navigation";
import { getExtracted } from "next-intl/server";
import { Suspense } from "react";
import { ArkMatchDetailContent } from "@/components/account-ark/ark-match-detail-content";
import { ArkMatchDetailLoadingState } from "@/components/account-ark/ark-match-detail-loading-state";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page({ params }: PageProps<"/account/ark/[matchId]">) {
  const user = await requireCurrentUserWithGovernor();
  const t = await getExtracted();
  const { matchId } = await params;
  const governorId = user.claimedGovernors[0]?.governorId;

  if (governorId == null) {
    redirect("/account/settings/governors");
  }

  return (
    <>
      <Heading>{t("Ark Match")}</Heading>
      <Suspense fallback={<ArkMatchDetailLoadingState />}>
        <ArkMatchDetailContent governorId={governorId} matchId={matchId} />
      </Suspense>
    </>
  );
}
