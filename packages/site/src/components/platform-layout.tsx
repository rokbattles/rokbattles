"use client";

import {
  ArrowDownTrayIcon,
  ChartPieIcon,
  FireIcon,
  FlagIcon,
  GiftIcon,
  MoonIcon,
  QuestionMarkCircleIcon,
  ScaleIcon,
  ShieldCheckIcon,
  SunIcon,
  TrophyIcon,
} from "@heroicons/react/16/solid";
import { useTheme } from "@wrksz/themes/client";
import { usePathname } from "next/navigation";
import { useExtracted } from "next-intl";
import type React from "react";
import { use, useCallback, useEffect, useState } from "react";
import { LanguageSelector } from "@/components/language-selector";
import { SidebarGovernorHeader } from "@/components/sidebar-governor-header";
import { Navbar } from "@/components/ui/navbar";
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
} from "@/components/ui/sidebar";
import { SidebarLayout } from "@/components/ui/sidebar-layout";
import { useCurrentUser } from "@/hooks/use-current-user";
import type { CurrentUser } from "@/lib/types/current-user";
import { GovernorContext } from "@/providers/governor-context";

type PlatformLayoutProps = {
  children: React.ReactNode;
  initialUser?: CurrentUser | null;
};

export function PlatformLayout({ children, initialUser }: PlatformLayoutProps) {
  const t = useExtracted();
  const pathname = usePathname();
  const { resolvedTheme, setTheme } = useTheme();
  const { user, loading, refresh } = useCurrentUser({ initialUser });
  const governorContext = use(GovernorContext);
  const [isMounted, setIsMounted] = useState(false);

  if (!governorContext) {
    throw new Error("PlatformLayout must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;
  const showGovernorSection = Boolean(!loading && user);
  const showMyReports = Boolean(!loading && user && activeGovernor);
  const isDark = isMounted ? resolvedTheme === "dark" : false;
  const ThemeIcon = isDark ? SunIcon : MoonIcon;
  const themeLabel = isMounted ? (isDark ? t("Light mode") : t("Dark mode")) : t("Theme");

  useEffect(() => {
    setIsMounted(true);
  }, []);

  const handleThemeToggle = useCallback(() => {
    setTheme(isDark ? "light" : "dark");
  }, [isDark, setTheme]);

  const handleLogout = useCallback(async () => {
    const response = await fetch("/proxy/v1/auth/logout", { method: "POST" });

    if (response.ok) {
      await refresh();
    } else {
      console.error("Failed to logout");
    }
  }, [refresh]);

  return (
    <SidebarLayout
      navbar={<Navbar />}
      sidebar={
        <Sidebar>
          {!loading && user ? (
            <SidebarGovernorHeader user={user} handleLogout={handleLogout} />
          ) : (
            <SidebarHeader>
              <SidebarItem disabled={true}>ROK Battles</SidebarItem>
            </SidebarHeader>
          )}
          <SidebarBody>
            <SidebarSection>
              <SidebarHeading>Community</SidebarHeading>
              <SidebarItem href="/" current={pathname === "/"}>
                <FireIcon />
                <SidebarLabel>{t("Battle Reports")}</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/olympian-arena" current={pathname === "/olympian-arena"}>
                <TrophyIcon />
                <SidebarLabel>{t("Olympian Arena")}</SidebarLabel>
              </SidebarItem>
              <SidebarItem
                href="/loot-explorer"
                current={pathname === "/loot-explorer" || pathname.startsWith("/loot-explorer/")}
              >
                <GiftIcon />
                <SidebarLabel>{t("Loot Explorer")}</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/combat-lab" current={pathname === "/combat-lab"}>
                <ChartPieIcon />
                <SidebarLabel>{t("Combat Lab")}</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
            {showGovernorSection && (
              <SidebarSection>
                <SidebarHeading>{t("My Data")}</SidebarHeading>
                {showMyReports ? (
                  <>
                    <SidebarItem href="/account/reports" current={pathname === "/account/reports"}>
                      <FireIcon />
                      <SidebarLabel>{t("My Battles")}</SidebarLabel>
                    </SidebarItem>
                    <SidebarItem
                      href="/account/pairings"
                      current={pathname === "/account/pairings"}
                    >
                      <ScaleIcon />
                      <SidebarLabel>{t("My Pairings")}</SidebarLabel>
                    </SidebarItem>
                    <SidebarItem
                      href="/account/loot"
                      current={
                        pathname === "/account/loot" || pathname.startsWith("/account/loot/")
                      }
                    >
                      <GiftIcon />
                      <SidebarLabel>{t("My Loot")}</SidebarLabel>
                    </SidebarItem>
                    <SidebarItem
                      href="/account/resources"
                      current={pathname === "/account/resources"}
                    >
                      <GiftIcon />
                      <SidebarLabel>{t("My Resources")}</SidebarLabel>
                    </SidebarItem>
                    <SidebarItem
                      href="/account/ark"
                      current={pathname === "/account/ark" || pathname.startsWith("/account/ark/")}
                    >
                      <FlagIcon />
                      <SidebarLabel>{t("My Ark Matches")}</SidebarLabel>
                    </SidebarItem>
                  </>
                ) : null}
              </SidebarSection>
            )}
            <SidebarSpacer />
            <SidebarSection>
              <SidebarItem href="/legal">
                <ShieldCheckIcon />
                <SidebarLabel>{t("Legal")}</SidebarLabel>
              </SidebarItem>
              <SidebarItem
                href="/discord"
                target="_blank"
                rel="noopener noreferrer"
                prefetch={false}
              >
                <QuestionMarkCircleIcon />
                <SidebarLabel>{t("Support")}</SidebarLabel>
              </SidebarItem>
              <SidebarItem
                href="/docs/installation"
                target="_blank"
                rel="noopener noreferrer"
                prefetch={false}
              >
                <ArrowDownTrayIcon />
                <SidebarLabel>{t("Desktop App")}</SidebarLabel>
              </SidebarItem>
              <SidebarItem onClick={handleThemeToggle} aria-label={t("Toggle theme")}>
                <ThemeIcon />
                <SidebarLabel>{themeLabel}</SidebarLabel>
              </SidebarItem>
              <LanguageSelector />
            </SidebarSection>
          </SidebarBody>
          {!user ? (
            <SidebarFooter className="max-lg:hidden">
              <SidebarItem href="/proxy/v1/auth/discord/login" prefetch={false}>
                <SidebarLabel>{t("Sign in")}</SidebarLabel>
              </SidebarItem>
            </SidebarFooter>
          ) : null}
        </Sidebar>
      }
    >
      {children}
    </SidebarLayout>
  );
}
