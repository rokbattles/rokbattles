import { AccountBindsContent } from "@/components/(app)/account/settings/governor-binds/account-binds-content";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { requireCurrentUser } from "@/lib/require-user";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page() {
  const user = await requireCurrentUser();

  return (
    <SettingsPage>
      <AccountBindsContent initialUser={user} />
    </SettingsPage>
  );
}
