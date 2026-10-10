"use client";

import { usePathname } from "next/navigation";
import { useExtracted } from "next-intl";
import { Navbar, NavbarItem, NavbarLabel, NavbarSection } from "@/components/ui/navbar";

export function AccountSettingsNav({ showOverlay }: { showOverlay: boolean }) {
  const t = useExtracted();
  const pathname = usePathname() ?? "";
  const settingsItems = [
    {
      href: "/account/settings",
      label: t("General"),
      isActive: (path: string) => path === "/account/settings",
    },
    {
      href: "/account/settings/governors",
      label: t("Governors"),
      isActive: (path: string) => path.startsWith("/account/settings/governors"),
    },
  ];
  if (showOverlay) {
    settingsItems.push({
      href: "/account/settings/overlay",
      label: t("Stream overlay"),
      isActive: (path: string) => path.startsWith("/account/settings/overlay"),
    });
  }

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
