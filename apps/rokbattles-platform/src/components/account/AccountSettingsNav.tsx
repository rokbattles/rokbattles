"use client";

import { usePathname } from "next/navigation";
import { Navbar, NavbarItem, NavbarLabel, NavbarSection } from "@/components/ui/Navbar";

const settingsItems = [
  {
    href: "/account/settings",
    label: "General",
    isActive: (pathname: string) => pathname === "/account/settings",
  },
  {
    href: "/account/settings/governors",
    label: "Governors",
    isActive: (pathname: string) => pathname.startsWith("/account/settings/governors"),
  },
];

export function AccountSettingsNav() {
  const pathname = usePathname() ?? "";

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
