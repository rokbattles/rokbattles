"use client";

import {
  ArrowDownTrayIcon,
  ArrowRightStartOnRectangleIcon,
  ArrowTrendingUpIcon,
  ChevronUpIcon,
  FireIcon,
  QuestionMarkCircleIcon,
  ScaleIcon,
  ShieldCheckIcon,
  TrophyIcon,
} from "@heroicons/react/16/solid";
import { usePathname } from "next/navigation";
import type React from "react";
import { useCallback, useContext } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";
import { SidebarGovernorHeader } from "@/components/SidebarGovernorHeader";
import { Avatar } from "@/components/ui/Avatar";
import {
  Dropdown,
  DropdownButton,
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
  );
}

export function PlatformLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const { user, loading, refresh } = useCurrentUser();
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("PlatformLayout must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;
  const showMyReports = Boolean(!loading && user && activeGovernor);

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
          {!loading && user ? <SidebarGovernorHeader user={user} onRefresh={refresh} /> : null}
          <SidebarBody>
            <SidebarSection>
              <SidebarItem href="/" current={pathname === "/"}>
                <FireIcon />
                <SidebarLabel>Explore Battles</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/olympian-arena" current={pathname === "/olympian-arena"}>
                <TrophyIcon />
                <SidebarLabel>Explore Duels</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/trends" current={pathname === "/trends"}>
                <ArrowTrendingUpIcon />
                <SidebarLabel>Explore Trends</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
            {!loading && user && showMyReports && (
              <SidebarSection>
                <SidebarHeading>Governor</SidebarHeading>
                <SidebarItem href="/my-reports" current={pathname === "/my-reports"}>
                  <FireIcon />
                  <SidebarLabel>My Battles</SidebarLabel>
                </SidebarItem>
                <SidebarItem href="/my-pairings" current={pathname === "/my-pairings"}>
                  <ScaleIcon />
                  <SidebarLabel>My Pairings</SidebarLabel>
                </SidebarItem>
              </SidebarSection>
            )}
            <SidebarSpacer />
            <SidebarSection>
              <SidebarItem
                href="/discord"
                target="_blank"
                rel="noopener noreferrer"
                prefetch={false}
              >
                <QuestionMarkCircleIcon />
                <SidebarLabel>Support</SidebarLabel>
              </SidebarItem>
              <SidebarItem
                href="/desktop-app"
                target="_blank"
                rel="noopener noreferrer"
                prefetch={false}
              >
                <ArrowDownTrayIcon />
                <SidebarLabel>Desktop App</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
          </SidebarBody>
          <SidebarFooter className="max-lg:hidden">
            {loading ? (
              <SidebarItem disabled>
                <SidebarLabel>Loading&hellip;</SidebarLabel>
              </SidebarItem>
            ) : user ? (
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
              <SidebarItem href="/api/auth/discord/login" prefetch={false}>
                <SidebarLabel>Sign in</SidebarLabel>
              </SidebarItem>
            )}
          </SidebarFooter>
        </Sidebar>
      }
    >
      {children}
    </SidebarLayout>
  );
}
