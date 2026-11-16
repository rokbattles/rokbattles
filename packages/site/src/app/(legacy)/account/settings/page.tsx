import { AccountEmail } from "@/components/(app)/account/settings/account-email";
import { DeleteAccount } from "@/components/(app)/account/settings/delete-account";
import { LoggedInDevicesLink } from "@/components/(app)/account/settings/logged-in-devices-link";
import { ProgramAccess } from "@/components/(app)/account/settings/program-access";
import { SettingsActionRow } from "@/components/(app)/account/settings/settings-action-row";
import { SettingsLink } from "@/components/(app)/account/settings/settings-link";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Divider } from "@/components/ui/divider";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUser();

  return (
    <SettingsPage>
      <SettingsSection title="Account Info">
        <AccountEmail email={user.email} />
      </SettingsSection>
      <Divider />
      <SettingsSection title="Security">
        <div>
          <LoggedInDevicesLink />
          <SettingsLink href="/account/settings/apps" label="Authorized Apps" value="0 apps" />
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
