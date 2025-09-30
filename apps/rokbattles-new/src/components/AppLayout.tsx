"use client";

import {
  ArrowTrendingUpIcon,
  ChevronDownIcon,
  FireIcon,
  PlusIcon,
  StarIcon,
} from "@heroicons/react/16/solid";
import React from "react";
import { Avatar } from "@/components/ui/Avatar";
import {
  Dropdown,
  DropdownButton,
  DropdownDescription,
  DropdownDivider,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/Dropdown";
import { SidebarLayout } from "@/components/ui/layout/SidebarLayout";
import {
  Sidebar,
  SidebarBody,
  SidebarHeader,
  SidebarHeading,
  SidebarItem,
  SidebarLabel,
  SidebarSection,
} from "@/components/ui/Sidebar";

export function AppLayout({ children }: { children: React.ReactNode }) {
  return (
    <SidebarLayout
      navbar={<React.Fragment />}
      sidebar={
        <Sidebar>
          <SidebarHeader>
            <Dropdown>
              <DropdownButton as={SidebarItem}>
                <Avatar
                  slot="icon"
                  src="https://plat-fau-global.lilithgame.com/p/astc/IM/10043/0/32624247/2025-08-05/0439DD1061F65AFF1ABAF81479C4267A_250x250.jpg"
                />
                <SidebarLabel>Griggasaurus</SidebarLabel>
                <ChevronDownIcon />
              </DropdownButton>
              <DropdownMenu className="min-w-80 lg:min-w-64" anchor="bottom start">
                <DropdownItem>
                  <Avatar
                    slot="icon"
                    src="https://plat-fau-global.lilithgame.com/p/astc/IM/10043/0/32624247/2025-08-05/0439DD1061F65AFF1ABAF81479C4267A_250x250.jpg"
                  />
                  <DropdownLabel>Griggasaurus</DropdownLabel>
                  <DropdownDescription>71738515</DropdownDescription>
                </DropdownItem>
                <DropdownItem>
                  <Avatar
                    slot="icon"
                    src="https://imimg.lilithcdn.com/roc/llc_avatar/121307126/23/08/22/9064f14bbe519ded_640x640.jpg"
                  />
                  <DropdownLabel>F3yst</DropdownLabel>
                  <DropdownDescription>121307126</DropdownDescription>
                </DropdownItem>
                <DropdownItem>
                  <PlusIcon />
                  <DropdownLabel>Claim governor&hellip;</DropdownLabel>
                </DropdownItem>
                <DropdownDivider />
                <DropdownItem disabled>
                  <PlusIcon />
                  <DropdownLabel>New group&hellip;</DropdownLabel>
                </DropdownItem>
              </DropdownMenu>
            </Dropdown>
          </SidebarHeader>
          <SidebarBody>
            <SidebarSection>
              <SidebarItem>
                <FireIcon />
                <SidebarLabel>Explore Battles</SidebarLabel>
              </SidebarItem>
              <SidebarItem>
                <ArrowTrendingUpIcon />
                <SidebarLabel>Explore Trends</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
            <SidebarSection>
              <SidebarHeading>Account</SidebarHeading>
              <SidebarItem>
                <FireIcon />
                <SidebarLabel>My Battles</SidebarLabel>
              </SidebarItem>
              <SidebarItem>
                <StarIcon />
                <SidebarLabel>My Favorites</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
          </SidebarBody>
        </Sidebar>
      }
    >
      {children}
    </SidebarLayout>
  );
}
