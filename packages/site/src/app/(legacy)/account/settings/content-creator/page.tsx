import { ProgramApplicationPrompt } from "@/components/(app)/account/settings/program-application-prompt";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { OverlaySettings } from "@/components/(app)/account/settings/stream-overlay/overlay-settings";
import { TextLink } from "@/components/ui/text";
import { getOverlayCommanders } from "@/lib/(app)/overlay-commanders";
import { hasRole } from "@/lib/(app)/roles";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUser();
  const member = hasRole(user, "content_creator");
  const commanders = getOverlayCommanders();
  const hasGovernor = user.claimedGovernors.length > 0;

  return (
    <SettingsPage>
      {member ? (
        <SettingsSection
          title="Stream Overlay"
          description={
            !hasGovernor ? (
              <>
                <TextLink href="/account/settings/game/governors">Bind a governor</TextLink> to
                configure your stream overlay.
              </>
            ) : undefined
          }
        >
          {hasGovernor ? (
            <OverlaySettings commanders={{ 1: commanders[1], 3: commanders[3] }} />
          ) : null}
        </SettingsSection>
      ) : (
        <SettingsSection title="Content Creator Program">
          <ProgramApplicationPrompt program="Content Creator" />
        </SettingsSection>
      )}
    </SettingsPage>
  );
}
