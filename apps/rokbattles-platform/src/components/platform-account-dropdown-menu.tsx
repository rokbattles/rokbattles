"use client";

import {
  ArrowRightStartOnRectangleIcon,
  Cog6ToothIcon,
  ScaleIcon,
} from "@heroicons/react/16/solid";
import { useExtracted } from "next-intl";
import { useState } from "react";
import { CookieConsentDialog } from "@/components/cookie-consent-dialog";
import {
  DropdownDivider,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/dropdown";

export function PlatformAccountDropdownMenu({
  anchor,
  handleLogout,
}: {
  anchor: "top start" | "bottom end";
  handleLogout: () => Promise<void>;
}) {
  const [isCookieDialogOpen, setIsCookieDialogOpen] = useState(false);
  const t = useExtracted();
  const tAccount = useExtracted();

  return (
    <>
      <DropdownMenu className="min-w-64" anchor={anchor}>
        <DropdownItem href="/account/settings">
          <Cog6ToothIcon />
          <DropdownLabel>{tAccount("Account Settings")}</DropdownLabel>
        </DropdownItem>
        <DropdownItem onClick={() => setIsCookieDialogOpen(true)}>
          <ScaleIcon />
          <DropdownLabel>{t("Cookie Settings")}</DropdownLabel>
        </DropdownItem>
        <DropdownDivider />
        <DropdownItem onClick={() => handleLogout()}>
          <ArrowRightStartOnRectangleIcon />
          <DropdownLabel>{t("Sign out")}</DropdownLabel>
        </DropdownItem>
      </DropdownMenu>
      <CookieConsentDialog open={isCookieDialogOpen} onClose={() => setIsCookieDialogOpen(false)} />
    </>
  );
}
