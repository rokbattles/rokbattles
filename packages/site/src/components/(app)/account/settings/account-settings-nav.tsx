"use client";

import { usePathname } from "next/navigation";
import { Navbar, NavbarItem, NavbarLabel, NavbarSection } from "@/components/ui/navbar";

const items = [
  { href: "/account/settings", label: "Account" },
  { href: "/account/settings/game", label: "Game" },
  { href: "/account/settings/content-creator", label: "Content Creator" },
  { href: "/account/settings/developer", label: "Developer" },
];

export function AccountSettingsNav() {
  const pathname = usePathname() ?? "";
  const active =
    items.slice(1).find((item) => pathname.startsWith(item.href))?.href ?? items[0].href;

  return (
    <Navbar aria-label="Account settings" className="border-b border-white/10">
      <NavbarSection className="flex-wrap gap-x-2 gap-y-5">
        {items.map((item) => (
          <NavbarItem
            key={item.href}
            href={item.href}
            current={active === item.href}
            aria-current={active === item.href ? "page" : undefined}
          >
            <NavbarLabel>{item.label}</NavbarLabel>
          </NavbarItem>
        ))}
      </NavbarSection>
    </Navbar>
  );
}
