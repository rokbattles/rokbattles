import { ArkMatchHistoryContent } from "@/components/account-ark/ark-match-history-content";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();

  return (
    <>
      <Heading>Ark Recap</Heading>
      <ArkMatchHistoryContent />
    </>
  );
}
