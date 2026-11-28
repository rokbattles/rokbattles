import { AuthorizedApps } from "@/components/(app)/account/settings/authorized-apps";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";

import { getAuthorizedApps } from "@/lib/(app)/authorized-apps";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page() {
  const apps = await getAuthorizedApps();

  return (
    <SettingsPage>
      <SettingsSection
        title="Authorized Apps"
        description="These include third-party services you’ve given permission to access your ROK Battles account. You can review what each app can do and revoke access anytime."
      >
        <AuthorizedApps apps={apps} />
      </SettingsSection>
    </SettingsPage>
  );
}
