import { AuthorizedApps } from "@/components/(app)/account/settings/authorized-apps";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";

export default function Page() {
  return (
    <SettingsPage backHref="/account/settings">
      <SettingsSection
        title="Authorized Apps"
        description="These include third-party services you’ve given permission to access your ROK Battles account. You can review what each app can do and revoke access anytime."
      >
        <AuthorizedApps apps={null} />
      </SettingsSection>
    </SettingsPage>
  );
}
