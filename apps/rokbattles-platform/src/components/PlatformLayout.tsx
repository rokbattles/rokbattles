"use client";

import {
  ArrowDownTrayIcon,
  ArrowTrendingUpIcon,
  ChevronUpIcon,
  FireIcon,
  MoonIcon,
  QuestionMarkCircleIcon,
  ScaleIcon,
  StarIcon,
  SunIcon,
  TrophyIcon,
} from "@heroicons/react/16/solid";
import { usePathname } from "next/navigation";
import { useTheme } from "next-themes";
import type React from "react";
import { useCallback, useContext, useEffect, useState } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";
import { PlatformAccountDropdownMenu } from "@/components/PlatformAccountDropdownMenu";
import { SidebarGovernorHeader } from "@/components/SidebarGovernorHeader";
import { Avatar } from "@/components/ui/Avatar";
import { Dropdown, DropdownButton } from "@/components/ui/Dropdown";
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
import type { CurrentUser } from "@/lib/types/current-user";

type PlatformLayoutProps = {
  children: React.ReactNode;
  initialUser?: CurrentUser | null;
};

export function PlatformLayout({ children, initialUser }: PlatformLayoutProps) {
  const pathname = usePathname();
  const { resolvedTheme, setTheme } = useTheme();
  const { user, loading, refresh } = useCurrentUser({ initialUser });
  const governorContext = useContext(GovernorContext);
  const [isMounted, setIsMounted] = useState(false);

  if (!governorContext) {
    throw new Error("PlatformLayout must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;
  const showGovernorSection = Boolean(!loading && user);
  const showMyReports = Boolean(!loading && user && activeGovernor);
  const isDark = isMounted ? resolvedTheme === "dark" : false;
  const ThemeIcon = isDark ? SunIcon : MoonIcon;
  const themeLabel = isMounted ? (isDark ? "Light mode" : "Dark mode") : "Theme";

  useEffect(() => {
    setIsMounted(true);
  }, []);

  const handleThemeToggle = useCallback(() => {
    setTheme(isDark ? "light" : "dark");
  }, [isDark, setTheme]);

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
                  <DropdownButton as={NavbarItem} aria-label="Open account menu">
                    <Avatar src={user.avatar} square />
                  </DropdownButton>
                  <PlatformAccountDropdownMenu anchor="bottom end" handleLogout={handleLogout} />
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
          {!loading && user ? <SidebarGovernorHeader user={user} /> : null}
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
            {showGovernorSection && (
              <SidebarSection>
                <SidebarHeading>Account</SidebarHeading>
                {showMyReports ? (
                  <>
                    <SidebarItem href="/account/reports" current={pathname === "/account/reports"}>
                      <FireIcon />
                      <SidebarLabel>My Battles</SidebarLabel>
                    </SidebarItem>
                    <SidebarItem
                      href="/account/pairings"
                      current={pathname === "/account/pairings"}
                    >
                      <ScaleIcon />
                      <SidebarLabel>My Pairings</SidebarLabel>
                    </SidebarItem>
                  </>
                ) : null}
                <SidebarItem href="/account/favorites" current={pathname === "/account/favorites"}>
                  <StarIcon />
                  <SidebarLabel>My Favorites</SidebarLabel>
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
              <SidebarItem onClick={handleThemeToggle} aria-label="Toggle theme">
                <ThemeIcon />
                <SidebarLabel>{themeLabel}</SidebarLabel>
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
                <PlatformAccountDropdownMenu anchor="top start" handleLogout={handleLogout} />
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
