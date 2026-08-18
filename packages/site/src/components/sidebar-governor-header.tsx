"use client";

import {
  ArrowRightStartOnRectangleIcon,
  ChevronDownIcon,
  Cog6ToothIcon,
  PlusIcon,
  ScaleIcon,
} from "@heroicons/react/16/solid";
import { useExtracted } from "next-intl";
import { use, useState } from "react";
import { CookieConsentDialog } from "@/components/cookie-consent-dialog";
import { Avatar } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import {
  Dropdown,
  DropdownButton,
  DropdownDivider,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/dropdown";
import { SidebarHeader, SidebarItem, SidebarLabel } from "@/components/ui/sidebar";
import { MAX_GOVERNOR_BINDS } from "@/lib/account-binds/constants";
import type { CurrentUser } from "@/lib/types/current-user";
import { GovernorContext } from "@/providers/governor-context";

type SidebarGovernorHeaderProps = {
  user: CurrentUser;
  handleLogout: () => Promise<void>;
};

export function SidebarGovernorHeader({ user, handleLogout }: SidebarGovernorHeaderProps) {
  const t = useExtracted();
  const [isCookieDialogOpen, setIsCookieDialogOpen] = useState(false);
  const context = use(GovernorContext);
  if (!context) {
    throw new Error("SidebarGovernorHeader must be used within a GovernorProvider");
  }

  const { activeGovernor, governors, selectGovernor } = context;

  const displayName = activeGovernor
    ? (activeGovernor.governorName ?? activeGovernor.governorId.toString())
    : t("Select Governor");
  const displayAvatar = activeGovernor?.governorAvatar ?? null;
  const canClaimMore = user.claimedGovernors.length < MAX_GOVERNOR_BINDS;

  return (
    <>
      <SidebarHeader>
        <Dropdown>
          <DropdownButton as={SidebarItem}>
            {displayAvatar && <Avatar slot="icon" src={displayAvatar} className="size-10" square />}
            <SidebarLabel>{displayName}</SidebarLabel>
            <ChevronDownIcon />
          </DropdownButton>
          <DropdownMenu className="min-w-80 lg:min-w-64" anchor="bottom start">
            {governors.length > 0 &&
              governors.map((claim) => (
                <DropdownItem
                  key={claim.governorId}
                  onClick={() => selectGovernor(claim.governorId)}
                >
                  {claim.governorAvatar && <Avatar slot="icon" src={claim.governorAvatar} square />}
                  <DropdownLabel>
                    {claim.governorName ? (
                      <>
                        {claim.governorName} <Badge className="ml-1">{claim.governorId}</Badge>
                      </>
                    ) : (
                      claim.governorId.toString()
                    )}
                  </DropdownLabel>
                </DropdownItem>
              ))}
            {canClaimMore ? (
              <DropdownItem href="/account/settings/governors">
                <PlusIcon />
                <DropdownLabel>{t("Claim governor")}</DropdownLabel>
              </DropdownItem>
            ) : null}
            <DropdownDivider />
            <DropdownItem href="/account/settings">
              <Cog6ToothIcon />
              <DropdownLabel>{t("Account Settings")}</DropdownLabel>
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
        </Dropdown>
      </SidebarHeader>
      <CookieConsentDialog open={isCookieDialogOpen} onClose={() => setIsCookieDialogOpen(false)} />
    </>
  );
}
