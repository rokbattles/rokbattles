import { LoggedInDevices } from "@/components/(app)/account/settings/logged-in-devices";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default function Page() {
  return (
    <SettingsPage>
      <SettingsSection
        title="Logged-in Devices"
        description="These are all the devices that are currently logged in with your ROK Battles account. Log out of devices that you don’t recognize."
      />
      <LoggedInDevices />
    </SettingsPage>
  );
}
