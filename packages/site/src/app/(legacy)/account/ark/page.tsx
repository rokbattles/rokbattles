import { redirect } from "next/navigation";

import { ArkMatchHistoryContent } from "@/components/account-ark/ark-match-history-content";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUserWithGovernor();

  const governorId = user.claimedGovernors[0]?.governorId;

  if (governorId == null) {
    redirect("/account/settings/governors");
  }

  return (
    <>
      <Heading>Ark Recap</Heading>
      <ArkMatchHistoryContent />
    </>
  );
}
