"use client";

import { usePathname } from "next/navigation";
import { Navbar, NavbarItem, NavbarLabel, NavbarSection } from "@/components/ui/navbar";

export function AppSettingsNav({ appId }: { appId: string }) {
  const pathname = usePathname();
  const generalHref = `/developer/apps/${appId}`;
  const items = [
    { href: generalHref, label: "General" },
    { href: `${generalHref}/authorization`, label: "Authorization" },
  ];

  return (
    <Navbar aria-label="App settings" className="border-b border-white/10">
      <NavbarSection className="flex-wrap gap-x-2 gap-y-5">
        {items.map(({ href, label }) => {
          const current = pathname === href;
          return (
            <NavbarItem
              key={href}
              href={href}
              current={current}
              aria-current={current ? "page" : undefined}
            >
              <NavbarLabel>{label}</NavbarLabel>
            </NavbarItem>
          );
        })}
      </NavbarSection>
    </Navbar>
  );
}
