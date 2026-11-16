import type { JSX } from "react";
import { SettingsSecretInput } from "@/components/(app)/account/settings/settings-secret-input";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Button } from "@/components/ui/button";
import { Text } from "@/components/ui/text";
import type { OverlayUrlAction } from "@/lib/(app)/types/overlay";

type OverlayConnectionProps = {
  url: string | null;
  enabled: boolean;
  busy: boolean;
  onConfirm: (action: OverlayUrlAction) => void;
};

export function OverlayConnection({
  url,
  enabled,
  busy,
  onConfirm,
}: OverlayConnectionProps): JSX.Element {
  return (
    <SettingsSection
      title="Browser source URLs"
      description="In OBS, Streamlabs, or other streaming software, add a Browser Source and paste the Live URL. Use the Test URL to position and size sample battles, then switch to the Live URL before streaming."
    >
      {url ? (
        <div className="space-y-5">
          <SettingsSecretInput key={url} label="Live URL" value={url} />
          <SettingsSecretInput key={`${url}/test`} label="Test URL" value={`${url}/test`} />
        </div>
      ) : null}
      {!url && !enabled ? (
        <Text>
          Create an overlay URL to connect your streaming software. You can explore the preview
          first.
        </Text>
      ) : null}
      {enabled ? (
        <div className="flex flex-wrap gap-3">
          <Button disabled={busy} onClick={() => onConfirm("rotate")}>
            Rotate URL
          </Button>
          <Button color="red" disabled={busy} onClick={() => onConfirm("disable")}>
            Disable
          </Button>
        </div>
      ) : null}
    </SettingsSection>
  );
}
