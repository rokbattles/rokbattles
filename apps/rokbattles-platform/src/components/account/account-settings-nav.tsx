"use client";

import { usePathname } from "next/navigation";
import { useTranslations } from "next-intl";
import { Navbar, NavbarItem, NavbarLabel, NavbarSection } from "@/components/ui/navbar";

export function AccountSettingsNav() {
  const t = useTranslations("account");
  const pathname = usePathname() ?? "";
  const settingsItems = [
    {
      href: "/account/settings",
      label: t("settings.nav.general"),
      isActive: (path: string) => path === "/account/settings",
    },
    {
      href: "/account/settings/governors",
      label: t("settings.nav.governors"),
      isActive: (path: string) => path.startsWith("/account/settings/governors"),
    },
  ];

  return (
    <Navbar className="gap-2">
      <NavbarSection>
        {settingsItems.map((item) => (
          <NavbarItem key={item.href} href={item.href} current={item.isActive(pathname)}>
            <NavbarLabel>{item.label}</NavbarLabel>
          </NavbarItem>
        ))}
      </NavbarSection>
    </Navbar>
  );
}
