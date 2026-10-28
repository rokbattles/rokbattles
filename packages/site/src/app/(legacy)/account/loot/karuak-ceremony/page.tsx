import { redirect } from "next/navigation";
import { PersonalLootContent } from "@/components/account-loot/personal-loot-content";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUserWithGovernor();
  if (user.claimedGovernors[0]?.governorId == null) {
    redirect("/account/settings/governors");
  }

  return <PersonalLootContent active="karuak-ceremony" endpoint="karuak-ceremony" />;
}
