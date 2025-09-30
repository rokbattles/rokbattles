"use client";

import {
  ArrowRightStartOnRectangleIcon,
  ArrowTrendingUpIcon,
  ChevronDownIcon,
  ChevronUpIcon,
  FireIcon,
  PlusIcon,
  QuestionMarkCircleIcon,
  StarIcon,
  UserCircleIcon,
} from "@heroicons/react/16/solid";
import type React from "react";
import { useCallback } from "react";
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
  Navbar,
  NavbarItem,
  NavbarLabel,
  NavbarSection,
  NavbarSpacer,
} from "@/components/ui/Navbar";
import {
  Sidebar,
  SidebarBody,
  SidebarFooter,
  SidebarHeader,
  SidebarHeading,
  SidebarItem,
  SidebarLabel,
  SidebarSection,
  SidebarSpacer,
} from "@/components/ui/Sidebar";
import { useCurrentUser } from "@/hooks/useCurrentUser";

function AccountDropdownMenu({
  anchor,
  handleLogout,
}: {
  anchor: "top start" | "bottom end";
  handleLogout: () => Promise<void>;
}) {
  return (
    <DropdownMenu className="min-w-64" anchor={anchor}>
      <DropdownItem>
        <UserCircleIcon />
        <DropdownLabel>My account</DropdownLabel>
      </DropdownItem>
      <DropdownDivider />
      <DropdownItem onClick={() => handleLogout()}>
        <ArrowRightStartOnRectangleIcon />
        <DropdownLabel>Sign out</DropdownLabel>
      </DropdownItem>
    </DropdownMenu>
  );
}

export function AppLayout({ children }: { children: React.ReactNode }) {
  const { user, loading, refresh } = useCurrentUser();

  const handleLogout = useCallback(async () => {
    const response = await fetch("/api/auth/logout", { method: "POST" });

    if (response.ok) {
      await refresh();
    } else {
      console.error("Failed to logout");
    }
  }, [refresh]);

  return (
    <SidebarLayout
      navbar={
        <Navbar>
          <NavbarSpacer />
          <NavbarSection>
            {!loading &&
              (user ? (
                <Dropdown>
                  <DropdownButton as={NavbarItem}>
                    <Avatar src={user.avatar} square />
                  </DropdownButton>
                  <AccountDropdownMenu anchor="bottom end" handleLogout={handleLogout} />
                </Dropdown>
              ) : (
                <NavbarItem href="/api/auth/discord/login">
                  <NavbarLabel>Sign in</NavbarLabel>
                </NavbarItem>
              ))}
          </NavbarSection>
        </Navbar>
      }
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
            <SidebarSpacer />
            <SidebarSection>
              <SidebarItem>
                <QuestionMarkCircleIcon />
                <SidebarLabel>Support</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
          </SidebarBody>
          <SidebarFooter className="max-lg:hidden">
            {!loading &&
              (user ? (
                <Dropdown>
                  <DropdownButton as={SidebarItem}>
                    <span className="flex min-w-0 items-center gap-3">
                      <Avatar src={user.avatar} className="size-10" square />
                      <span className="min-w-0">
                        <span className="block truncate text-sm/5 font-medium text-zinc-950 dark:text-white">
                          {user.globalName ?? user.username}
                        </span>
                        <span className="block truncate text-xs/5 font-normal text-zinc-500 dark:text-zinc-400">
                          {user.email}
                        </span>
                      </span>
                    </span>
                    <ChevronUpIcon />
                  </DropdownButton>
                  <AccountDropdownMenu anchor="top start" handleLogout={handleLogout} />
                </Dropdown>
              ) : (
                <SidebarItem href="/api/auth/discord/login">
                  <SidebarLabel>Sign in</SidebarLabel>
                </SidebarItem>
              ))}
          </SidebarFooter>
        </Sidebar>
      }
    >
      {children}
    </SidebarLayout>
  );
}
