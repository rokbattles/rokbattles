import { getTranslations } from "next-intl/server";
import type React from "react";
import { AccountSettingsNav } from "@/components/account/account-settings-nav";
import { Heading } from "@/components/ui/Heading";

export default async function Layout({ children }: { children: React.ReactNode }) {
  const t = await getTranslations("account");
  return (
    <div className="space-y-4">
      <div>
        <Heading>{t("titles.settings")}</Heading>
      </div>
      <AccountSettingsNav />
      {children}
    </div>
  );
}
