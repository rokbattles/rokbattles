"use client";

import { usePathname } from "next/navigation";

import { Navbar, NavbarItem, NavbarLabel, NavbarSection } from "@/components/ui/navbar";

export function AccountSettingsNav({ showOverlay }: { showOverlay: boolean }) {
  const pathname = usePathname() ?? "";
  const settingsItems = [
    {
      href: "/account/settings",
      label: "General",
      isActive: (path: string) => path === "/account/settings",
    },
    {
      href: "/account/settings/governors",
      label: "Governors",
      isActive: (path: string) => path.startsWith("/account/settings/governors"),
    },
  ];
  if (showOverlay) {
    settingsItems.push({
      href: "/account/settings/overlay",
      label: "Stream overlay",
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
