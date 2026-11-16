import { X } from "lucide-react";
import { SettingsEmptyState } from "@/components/(app)/account/settings/settings-empty-state";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Button } from "@/components/ui/button";
import { Divider } from "@/components/ui/divider";

export type LoggedInDevice = { id: string };

export function LoggedInDevices({ otherDevices }: { otherDevices: LoggedInDevice[] | null }) {
  return (
    <>
      <SettingsSection title="Current Device">
        <div className="py-3 text-sm">Unknown</div>
      </SettingsSection>
      <Divider />
      <SettingsSection title="Other Devices">
        {!otherDevices?.length ? (
          <SettingsEmptyState>No other devices.</SettingsEmptyState>
        ) : (
          <ul className="divide-y divide-white/10">
            {otherDevices.map((device) => (
              <li key={device.id} className="flex items-center justify-between gap-4 py-3 text-sm">
                <span>Unknown</span>
                <Button plain disabled aria-label="Log out device">
                  <X aria-hidden="true" className="size-4" />
                </Button>
              </li>
            ))}
          </ul>
        )}
      </SettingsSection>
    </>
  );
}
