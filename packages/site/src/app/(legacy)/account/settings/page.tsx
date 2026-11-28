import { AccountEmail } from "@/components/(app)/account/settings/account-email";
import { DeleteAccount } from "@/components/(app)/account/settings/delete-account";
import { LoggedInDevicesLink } from "@/components/(app)/account/settings/logged-in-devices-link";
import { ProgramAccess } from "@/components/(app)/account/settings/program-access";
import { SettingsActionRow } from "@/components/(app)/account/settings/settings-action-row";
import { SettingsLink } from "@/components/(app)/account/settings/settings-link";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Divider } from "@/components/ui/divider";
import { getAuthorizedApps } from "@/lib/(app)/authorized-apps";
import { requireCurrentUser } from "@/lib/require-user";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page() {
  const [user, apps] = await Promise.all([requireCurrentUser(), getAuthorizedApps()]);
  const appCount = apps ? `${apps.length} ${apps.length === 1 ? "app" : "apps"}` : undefined;

  return (
    <SettingsPage>
      <SettingsSection title="Account Info">
        <AccountEmail email={user.email} />
      </SettingsSection>
      <Divider />
      <SettingsSection title="Security">
        <div>
          <LoggedInDevicesLink />
          <SettingsLink href="/account/settings/apps" label="Authorized Apps" value={appCount} />
        </div>
      </SettingsSection>
      <Divider />
      <SettingsSection title="Program Access">
        <ProgramAccess user={user} />
      </SettingsSection>
      <Divider />
      <SettingsActionRow
        label="Close your account"
        description="Permanently close your account."
        inset
      >
        <DeleteAccount />
      </SettingsActionRow>
    </SettingsPage>
  );
}
