import { SettingsLink } from "@/components/(app)/account/settings/settings-link";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { requireCurrentUser } from "@/lib/require-user";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

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
