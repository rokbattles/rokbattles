"use client";

import {
  ArrowDownTrayIcon,
  ArrowTrendingUpIcon,
  ChevronUpIcon,
  FireIcon,
  GiftIcon,
  MoonIcon,
  QuestionMarkCircleIcon,
  ScaleIcon,
  StarIcon,
  SunIcon,
  TrophyIcon,
} from "@heroicons/react/16/solid";
import { usePathname } from "next/navigation";
import { useTranslations } from "next-intl";
import { useTheme } from "next-themes";
import type React from "react";
import { useCallback, useContext, useEffect, useState } from "react";
import { LanguageSelector } from "@/components/language-selector";
import { PlatformAccountDropdownMenu } from "@/components/platform-account-dropdown-menu";
import { SidebarGovernorHeader } from "@/components/sidebar-governor-header";
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
import { GovernorContext } from "@/providers/governor-context";

type PlatformLayoutProps = {
  children: React.ReactNode;
  initialUser?: CurrentUser | null;
};

export function PlatformLayout({ children, initialUser }: PlatformLayoutProps) {
  const t = useTranslations("navigation");
  const tAccount = useTranslations("account");
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
  const themeLabel = isMounted ? (isDark ? t("lightMode") : t("darkMode")) : t("theme");

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
                  <DropdownButton as={NavbarItem} aria-label={t("openAccountMenu")}>
                    <Avatar src={user.avatar} square />
                  </DropdownButton>
                  <PlatformAccountDropdownMenu anchor="bottom end" handleLogout={handleLogout} />
                </Dropdown>
              ) : (
                <NavbarItem href="/api/auth/discord/login">
                  <NavbarLabel>{t("signIn")}</NavbarLabel>
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
                <SidebarLabel>{t("exploreBattles")}</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/olympian-arena" current={pathname === "/olympian-arena"}>
                <TrophyIcon />
                <SidebarLabel>{t("exploreDuels")}</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/trends" current={pathname === "/trends"}>
                <ArrowTrendingUpIcon />
                <SidebarLabel>{t("exploreTrends")}</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
            {showGovernorSection && (
              <SidebarSection>
                <SidebarHeading>{t("account")}</SidebarHeading>
                {showMyReports ? (
                  <>
                    <SidebarItem href="/account/reports" current={pathname === "/account/reports"}>
                      <FireIcon />
                      <SidebarLabel>{t("myBattles")}</SidebarLabel>
                    </SidebarItem>
                    <SidebarItem
                      href="/account/pairings"
                      current={pathname === "/account/pairings"}
                    >
                      <ScaleIcon />
                      <SidebarLabel>{tAccount("titles.pairings")}</SidebarLabel>
                    </SidebarItem>
                    <SidebarItem href="/account/rewards" current={pathname === "/account/rewards"}>
                      <GiftIcon />
                      <SidebarLabel>{tAccount("titles.rewards")}</SidebarLabel>
                    </SidebarItem>
                  </>
                ) : null}
                <SidebarItem href="/account/favorites" current={pathname === "/account/favorites"}>
                  <StarIcon />
                  <SidebarLabel>{t("myFavorites")}</SidebarLabel>
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
                <SidebarLabel>{t("support")}</SidebarLabel>
              </SidebarItem>
              <SidebarItem
                href="/desktop-app"
                target="_blank"
                rel="noopener noreferrer"
                prefetch={false}
              >
                <ArrowDownTrayIcon />
                <SidebarLabel>{t("desktopApp")}</SidebarLabel>
              </SidebarItem>
              <SidebarItem onClick={handleThemeToggle} aria-label={t("toggleTheme")}>
                <ThemeIcon />
                <SidebarLabel>{themeLabel}</SidebarLabel>
              </SidebarItem>
              <LanguageSelector />
            </SidebarSection>
          </SidebarBody>
          <SidebarFooter className="max-lg:hidden">
            {loading ? (
              <SidebarItem disabled>
                <SidebarLabel>{t("loading")}</SidebarLabel>
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
                <SidebarLabel>{t("signIn")}</SidebarLabel>
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
