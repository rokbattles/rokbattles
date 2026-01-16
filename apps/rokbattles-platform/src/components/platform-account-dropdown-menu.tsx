"use client";

import {
  ArrowRightStartOnRectangleIcon,
  Cog6ToothIcon,
  ScaleIcon,
  ShieldCheckIcon,
} from "@heroicons/react/16/solid";
import { useTranslations } from "next-intl";
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
  const t = useTranslations("navigation");
  const tAccount = useTranslations("account");

  return (
    <>
      <DropdownMenu className="min-w-64" anchor={anchor}>
        <DropdownItem href="/account/settings">
          <Cog6ToothIcon />
          <DropdownLabel>{tAccount("titles.settings")}</DropdownLabel>
        </DropdownItem>
        <DropdownItem onClick={() => setIsCookieDialogOpen(true)}>
          <ScaleIcon />
          <DropdownLabel>{t("cookieSettings")}</DropdownLabel>
        </DropdownItem>
        <DropdownItem
          href="https://rokbattles.com/legal/privacy-policy"
          target="_blank"
          rel="noopener noreferrer"
        >
          <ShieldCheckIcon />
          <DropdownLabel>{t("privacyPolicy")}</DropdownLabel>
        </DropdownItem>
        <DropdownDivider />
        <DropdownItem onClick={() => handleLogout()}>
          <ArrowRightStartOnRectangleIcon />
          <DropdownLabel>{t("signOut")}</DropdownLabel>
        </DropdownItem>
      </DropdownMenu>
      <CookieConsentDialog open={isCookieDialogOpen} onClose={() => setIsCookieDialogOpen(false)} />
    </>
  );
}
