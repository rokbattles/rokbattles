import type { ReactNode } from "react";
import { AccountSettingsNav } from "@/components/(app)/account/settings/account-settings-nav";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUser } from "@/lib/require-user";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Layout({ children }: { children: ReactNode }) {
  await requireCurrentUser();

  return (
    <div className="space-y-4">
      <Heading>Account Settings</Heading>
      <AccountSettingsNav />
      {children}
    </div>
  );
}
