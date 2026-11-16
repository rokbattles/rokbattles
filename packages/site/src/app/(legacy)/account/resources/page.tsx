import { AccountResourcesContent } from "@/components/account-resources/account-resources-content";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();

  return (
    <>
      <Heading>My Resources</Heading>
      <AccountResourcesContent />
    </>
  );
}
