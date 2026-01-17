"use client";

import { ChevronDownIcon, PlusIcon } from "@heroicons/react/16/solid";
import { useTranslations } from "next-intl";
import { useContext } from "react";
import { Avatar } from "@/components/ui/avatar";
import {
  Dropdown,
  DropdownButton,
  DropdownDescription,
  DropdownDivider,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/dropdown";
import { SidebarHeader, SidebarItem, SidebarLabel } from "@/components/ui/sidebar";
import type { CurrentUser } from "@/lib/types/current-user";
import { GovernorContext } from "@/providers/governor-context";

type SidebarGovernorHeaderProps = {
  user: CurrentUser;
};

export function SidebarGovernorHeader({ user }: SidebarGovernorHeaderProps) {
  const t = useTranslations("navigation");
  const context = useContext(GovernorContext);
  if (!context) {
    throw new Error("SidebarGovernorHeader must be used within a GovernorProvider");
  }

  const { activeGovernor, governors, selectGovernor } = context;

  const displayName = activeGovernor
    ? (activeGovernor.governorName ?? activeGovernor.governorId.toString())
    : t("selectGovernor");
  const displayAvatar = activeGovernor?.governorAvatar ?? null;
  const canClaimMore = user.claimedGovernors.length < 3;

  return (
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
              <DropdownItem key={claim.governorId} onClick={() => selectGovernor(claim.governorId)}>
                {claim.governorAvatar && <Avatar slot="icon" src={claim.governorAvatar} square />}
                <DropdownLabel>{claim.governorName ?? claim.governorId.toString()}</DropdownLabel>
                {claim.governorName && (
                  <DropdownDescription>{claim.governorId}</DropdownDescription>
                )}
              </DropdownItem>
            ))}
          {canClaimMore ? (
            <DropdownItem href="/account/settings/governors">
              <PlusIcon />
              <DropdownLabel>{t("claimGovernor")}</DropdownLabel>
            </DropdownItem>
          ) : null}
          <DropdownDivider />
          <DropdownItem disabled>
            <PlusIcon />
            <DropdownLabel>{t("newGroup")}</DropdownLabel>
          </DropdownItem>
        </DropdownMenu>
      </Dropdown>
    </SidebarHeader>
  );
}
