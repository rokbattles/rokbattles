import { AccountBindsContent } from "@/components/(app)/account/settings/governor-binds/account-binds-content";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUser();

  return (
    <SettingsPage backHref="/account/settings/game" backLabel="Game">
      <AccountBindsContent initialUser={user} />
    </SettingsPage>
  );
}
