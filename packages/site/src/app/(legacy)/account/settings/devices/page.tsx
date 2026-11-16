import { LoggedInDevices } from "@/components/(app)/account/settings/logged-in-devices";
import { SettingsActionRow } from "@/components/(app)/account/settings/settings-action-row";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Button } from "@/components/ui/button";
import { Divider } from "@/components/ui/divider";

export default function Page() {
  return (
    <SettingsPage backHref="/account/settings">
      <SettingsSection
        title="Logged-in Devices"
        description="These are all the devices that are currently logged in with your ROK Battles account. Log out of devices that you don’t recognize."
      />
      <LoggedInDevices otherDevices={null} />
      <Divider />
      <SettingsActionRow
        label="Log out of all known devices"
        description="You’ll have to log back in on all logged out devices."
      >
        <Button color="red" disabled>
          Log out all known devices
        </Button>
      </SettingsActionRow>
    </SettingsPage>
  );
}
