import { AccountEmail } from "@/components/(app)/account/settings/account-email";
import { DeleteAccount } from "@/components/(app)/account/settings/delete-account";
import { LoggedInDevicesLink } from "@/components/(app)/account/settings/logged-in-devices-link";
import { SettingsActionRow } from "@/components/(app)/account/settings/settings-action-row";
import { SettingsLink } from "@/components/(app)/account/settings/settings-link";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Divider } from "@/components/ui/divider";
import { hasRole } from "@/lib/(app)/roles";
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
        <dl>
          {(
            [
              { label: "Content Creator", role: "content_creator" },
              { label: "Developer Program", role: "developer_program" },
            ] as const
          ).map(({ label, role }) => (
            <div key={role} className="flex items-center justify-between gap-4 px-3 py-3">
              <dt className="text-sm">{label}</dt>
              <dd>
                {hasRole(user, role) ? (
                  <Badge color="green">Active</Badge>
                ) : (
                  <Button outline disabled aria-label={`Apply to ${label}`}>
                    Apply
                  </Button>
                )}
              </dd>
            </div>
          ))}
        </dl>
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
