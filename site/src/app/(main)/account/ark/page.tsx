import { redirect } from "next/navigation";
import { getExtracted } from "next-intl/server";
import { Suspense } from "react";
import { ArkMatchHistoryContent } from "@/components/account-ark/ark-match-history-content";
import { ArkMatchHistoryLoadingState } from "@/components/account-ark/ark-match-history-loading-state";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUserWithGovernor();
  const t = await getExtracted();
  const governorId = user.claimedGovernors[0]?.governorId;

  if (governorId == null) {
    redirect("/account/settings/governors");
  }

  return (
    <>
      <Heading>{t("Ark Match History")}</Heading>
      <Suspense fallback={<ArkMatchHistoryLoadingState />}>
        <ArkMatchHistoryContent governorId={governorId} />
      </Suspense>
    </>
  );
}
