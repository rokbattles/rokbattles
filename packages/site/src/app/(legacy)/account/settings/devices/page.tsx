import { LoggedInDevices } from "@/components/(app)/account/settings/logged-in-devices";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";

export default function Page() {
  return (
    <SettingsPage backHref="/account/settings">
      <SettingsSection
        title="Logged-in Devices"
        description="These are all the devices that are currently logged in with your ROK Battles account. Log out of devices that you don’t recognize."
      />
      <LoggedInDevices />
    </SettingsPage>
  );
}
