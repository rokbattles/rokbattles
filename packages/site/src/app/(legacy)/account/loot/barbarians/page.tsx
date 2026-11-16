import { PersonalLootContent } from "@/components/account-loot/personal-loot-content";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();

  return <PersonalLootContent active="barbarians" endpoint="barbarians" />;
}
