import { getExtracted } from "next-intl/server";
import type React from "react";
import { AccountSettingsNav } from "@/components/account/account-settings-nav";
import { Heading } from "@/components/ui/heading";
import { getCurrentUser } from "@/lib/current-user";
import { hasOverlayAccess } from "@/lib/overlay-access";

export default async function Layout({ children }: { children: React.ReactNode }) {
  const [t, user] = await Promise.all([getExtracted(), getCurrentUser()]);
  return (
    <div className="space-y-4">
      <div>
        <Heading>{t("Account Settings")}</Heading>
      </div>
      <AccountSettingsNav showOverlay={hasOverlayAccess(user?.discordId)} />
      {children}
    </div>
  );
}
