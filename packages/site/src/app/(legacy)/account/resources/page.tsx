import { redirect } from "next/navigation";

import { AccountResourcesContent } from "@/components/account-resources/account-resources-content";
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
      <Heading>My Resources</Heading>
      <AccountResourcesContent />
    </>
  );
}
