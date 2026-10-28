import { MyPairingsContent } from "@/components/my-pairings/my-pairings-content";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();

  return (
    <>
      <Heading>My Pairings</Heading>
      <MyPairingsContent />
    </>
  );
}
