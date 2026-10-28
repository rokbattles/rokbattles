"use client";

import {
  BookOpen,
  ChartPie,
  CircleHelp,
  Flag,
  Flame,
  Gift,
  Map as MapIcon,
  Scale,
  Trophy,
  Warehouse,
} from "lucide-react";
import Image from "next/image";
import { usePathname } from "next/navigation";

import type React from "react";
import { use, useCallback } from "react";
import { DonateDialog } from "@/components/donate-dialog";
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

const fullWidthRoutes = new Set([
  "/combat-lab",
  "/combat-lab/rankings",
  "/combat-lab/presoc",
  "/combat-lab/presoc/rankings",
]);

export function PlatformLayout({ children, initialUser }: PlatformLayoutProps) {
  const pathname = usePathname();
  const { user, loading, refresh } = useCurrentUser({ initialUser });
  const governorContext = use(GovernorContext);

  if (!governorContext) {
    throw new Error("PlatformLayout must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;
  const showGovernorSection = Boolean(!loading && user);
  const showMyReports = Boolean(!loading && user && activeGovernor);

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
      fullWidth={fullWidthRoutes.has(pathname)}
      navbar={<Navbar />}
      sidebar={
        <Sidebar>
          {!loading && user ? (
            <SidebarGovernorHeader user={user} handleLogout={handleLogout} />
          ) : (
            <SidebarHeader>
              <div className="px-2">
                <Image
                  src="/assets/logo.svg"
                  alt="ROK Battles"
                  loading="eager"
                  width={2104}
                  height={556}
                  className="h-auto w-40"
                />
              </div>
            </SidebarHeader>
          )}
          <SidebarBody>
            <SidebarSection>
              <SidebarHeading>Community</SidebarHeading>
              <SidebarItem href="/" current={pathname === "/"}>
                <Flame data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Battle Reports</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/olympian-arena" current={pathname === "/olympian-arena"}>
                <Trophy data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Olympian Arena</SidebarLabel>
              </SidebarItem>
              <SidebarItem
                href="/loot-explorer"
                current={pathname === "/loot-explorer" || pathname.startsWith("/loot-explorer/")}
              >
                <Gift data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Loot Explorer</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/combat-lab" current={pathname.startsWith("/combat-lab")}>
                <ChartPie data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Combat Lab</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/territory-lab" current={pathname.startsWith("/territory-lab")}>
                <MapIcon data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Territory Lab</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
            {showGovernorSection && (
              <SidebarSection>
                <SidebarHeading>My Data</SidebarHeading>
                {showMyReports ? (
                  <>
                    <SidebarItem href="/account/reports" current={pathname === "/account/reports"}>
                      <Flame data-slot="icon" aria-hidden="true" />
                      <SidebarLabel>My Battles</SidebarLabel>
                    </SidebarItem>
                    <SidebarItem
                      href="/account/pairings"
                      current={pathname === "/account/pairings"}
                    >
                      <Scale data-slot="icon" aria-hidden="true" />
                      <SidebarLabel>My Pairings</SidebarLabel>
                    </SidebarItem>
                    <SidebarItem
                      href="/account/loot"
                      current={
                        pathname === "/account/loot" || pathname.startsWith("/account/loot/")
                      }
                    >
                      <Gift data-slot="icon" aria-hidden="true" />
                      <SidebarLabel>My Loot</SidebarLabel>
                    </SidebarItem>
                    <SidebarItem
                      href="/account/resources"
                      current={pathname === "/account/resources"}
                    >
                      <Warehouse data-slot="icon" aria-hidden="true" />
                      <SidebarLabel>My Resources</SidebarLabel>
                    </SidebarItem>
                    <SidebarItem
                      href="/account/ark"
                      current={pathname === "/account/ark" || pathname.startsWith("/account/ark/")}
                    >
                      <Flag data-slot="icon" aria-hidden="true" />
                      <SidebarLabel>Ark Recap</SidebarLabel>
                    </SidebarItem>
                  </>
                ) : null}
              </SidebarSection>
            )}
            <SidebarSpacer />
            <SidebarSection>
              <DonateDialog />
              <SidebarItem
                href="/discord"
                target="_blank"
                rel="noopener noreferrer"
                prefetch={false}
              >
                <CircleHelp data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Support</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/docs" target="_blank" rel="noopener noreferrer" prefetch={false}>
                <BookOpen data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Documentation</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
          </SidebarBody>
          {!user ? (
            <SidebarFooter>
              <SidebarItem href="/proxy/v1/auth/discord/login" prefetch={false}>
                <SidebarLabel>Sign in</SidebarLabel>
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
