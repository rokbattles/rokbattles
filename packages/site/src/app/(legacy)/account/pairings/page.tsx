import { MyPairingsContent } from "@/components/my-pairings/my-pairings-content";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();

  return <MyPairingsContent />;
}
