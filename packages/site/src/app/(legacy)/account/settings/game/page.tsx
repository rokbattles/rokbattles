import { SettingsLink } from "@/components/(app)/account/settings/settings-link";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUser();
  const count = user.claimedGovernors.length;

  return (
    <SettingsPage>
      <SettingsSection title="Game">
        <SettingsLink
          href="/account/settings/game/governors"
          label="Governor Binds"
          value={`${count} ${count === 1 ? "bind" : "binds"}`}
        />
      </SettingsSection>
    </SettingsPage>
  );
}
