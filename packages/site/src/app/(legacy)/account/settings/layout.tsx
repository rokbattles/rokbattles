import type { ReactNode } from "react";
import { AccountSettingsNav } from "@/components/(app)/account/settings/account-settings-nav";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUser } from "@/lib/require-user";

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
