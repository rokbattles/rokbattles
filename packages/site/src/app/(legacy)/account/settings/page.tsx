import { AccountEmail } from "@/components/(app)/account/settings/account-email";
import { SettingsActionRow } from "@/components/(app)/account/settings/settings-action-row";
import { SettingsLink } from "@/components/(app)/account/settings/settings-link";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Button } from "@/components/ui/button";
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
          <SettingsLink
            href="/account/settings/devices"
            label="Logged-in Devices"
            value="0 devices"
          />
          <SettingsLink href="/account/settings/apps" label="Authorized Apps" value="0 apps" />
        </div>
      </SettingsSection>
      <Divider />
      <SettingsActionRow
        label="Close your account"
        description="Permanently close your account."
        inset
      >
        <Button color="red" disabled>
          Delete account
        </Button>
      </SettingsActionRow>
    </SettingsPage>
  );
}
