"use client";

import {
  ArrowRightStartOnRectangleIcon,
  Cog6ToothIcon,
  ScaleIcon,
  ShieldCheckIcon,
} from "@heroicons/react/16/solid";
import { useState } from "react";
import { CookieConsentDialog } from "@/components/CookieConsentDialog";
import {
  DropdownDivider,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/Dropdown";

export function PlatformAccountDropdownMenu({
  anchor,
  handleLogout,
}: {
  anchor: "top start" | "bottom end";
  handleLogout: () => Promise<void>;
}) {
  const [isCookieDialogOpen, setIsCookieDialogOpen] = useState(false);

  return (
    <>
      <DropdownMenu className="min-w-64" anchor={anchor}>
        <DropdownItem href="/account/settings">
          <Cog6ToothIcon />
          <DropdownLabel>Account Settings</DropdownLabel>
        </DropdownItem>
        <DropdownItem onClick={() => setIsCookieDialogOpen(true)}>
          <ScaleIcon />
          <DropdownLabel>Cookie Settings</DropdownLabel>
        </DropdownItem>
        <DropdownItem
          href="https://rokbattles.com/legal/privacy-policy"
          target="_blank"
          rel="noopener noreferrer"
        >
          <ShieldCheckIcon />
          <DropdownLabel>Privacy Policy</DropdownLabel>
        </DropdownItem>
        <DropdownDivider />
        <DropdownItem onClick={() => handleLogout()}>
          <ArrowRightStartOnRectangleIcon />
          <DropdownLabel>Sign out</DropdownLabel>
        </DropdownItem>
      </DropdownMenu>
      <CookieConsentDialog open={isCookieDialogOpen} onClose={() => setIsCookieDialogOpen(false)} />
    </>
  );
}
