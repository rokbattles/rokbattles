import { redirect } from "next/navigation";
import { OverlaySettings } from "@/components/(app)/creators/overlay/overlay-settings";
import { Heading } from "@/components/ui/heading";
import { Text, TextLink } from "@/components/ui/text";
import { getOverlayCommanders } from "@/lib/(app)/overlay-commanders";
import { hasRole } from "@/lib/(app)/roles";
import { requireCurrentUser } from "@/lib/require-user";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page() {
  const user = await requireCurrentUser();
  if (!hasRole(user, "content_creator")) {
    redirect("/account/settings");
  }

  const commanders = getOverlayCommanders();
  const hasGovernor = user.claimedGovernors.length > 0;

  return (
    <div className="space-y-8">
      <Heading>Stream Overlay</Heading>
      {hasGovernor ? (
        <OverlaySettings commanders={{ 1: commanders[1], 3: commanders[3] }} />
      ) : (
        <Text>
          <TextLink href="/account/settings/game/governors">Bind a governor</TextLink> to configure
          your stream overlay.
        </Text>
      )}
    </div>
  );
}
