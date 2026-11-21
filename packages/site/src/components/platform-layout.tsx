"use client";

import {
  BookOpen,
  ChartPie,
  CircleHelp,
  Code2,
  Flag,
  Flame,
  Gift,
  Heart,
  LogIn,
  Map as MapIcon,
  MonitorPlay,
  Scale,
  Shield,
  Store,
  Swords,
  Trophy,
  Warehouse,
} from "lucide-react";
import Image from "next/image";
import { usePathname } from "next/navigation";

import type React from "react";
import { use, useCallback, useState } from "react";
import { PlatformBreadcrumbs } from "@/components/platform-breadcrumbs";
import { SidebarGovernorHeader } from "@/components/sidebar-governor-header";
import {
  Sidebar,
  SidebarBody,
  SidebarCategory,
  SidebarFooter,
  SidebarHeader,
  SidebarItem,
  SidebarLabel,
  SidebarSection,
  SidebarSpacer,
  SidebarSubnav,
} from "@/components/ui/sidebar";
import { SidebarLayout } from "@/components/ui/sidebar-layout";
import { useCurrentUser } from "@/hooks/use-current-user";
import { hasRole } from "@/lib/(app)/roles";
import type { CurrentUser } from "@/lib/types/current-user";
import { GovernorContext } from "@/providers/governor-context";

type PlatformLayoutProps = {
  children: React.ReactNode;
  initialUser?: CurrentUser | null;
};

type SidebarCategoryLabel =
  | "Community"
  | "Tools"
  | "My Data"
  | "Content Creators"
  | "Developer Program";

const myDataLinks = [
  { label: "Battles", href: "/account/reports", icon: Flame },
  { label: "Pairings", href: "/account/pairings", icon: Scale },
  { label: "Loot", href: "/account/loot", icon: Gift },
  { label: "Resources", href: "/account/resources", icon: Warehouse },
  { label: "Trading Post", href: "/account/trading-post", icon: Store },
  { label: "Ark Recap", href: "/account/ark", icon: Flag },
];

const programLinks = [
  {
    category: "Content Creators",
    role: "content_creator",
    label: "Stream Overlay",
    href: "/creators/overlay",
    icon: MonitorPlay,
  },
  {
    category: "Developer Program",
    role: "developer_program",
    label: "Apps",
    href: "/developer/apps",
    icon: Code2,
  },
] as const;

function currentLocation(pathname: string, href: string): boolean | "location" {
  if (pathname === href) return true;
  return pathname.startsWith(`${href}/`) ? "location" : false;
}

export function PlatformLayout({ children, initialUser }: PlatformLayoutProps) {
  const pathname = usePathname();
  const { user, loading, refresh } = useCurrentUser({ initialUser });
  const governorContext = use(GovernorContext);
  const [collapsedCategories, setCollapsedCategories] = useState<
    Partial<Record<SidebarCategoryLabel, boolean>>
  >({});
  const [combatLabOpen, setCombatLabOpen] = useState<boolean>();

  if (!governorContext) {
    throw new Error("PlatformLayout must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;
  const signedInUser = loading ? null : user;
  const combatLabCurrent = currentLocation(pathname, "/combat-lab");
  const preSocCurrent = currentLocation(pathname, "/combat-lab/presoc");
  const showCombatLabSubnav = combatLabOpen ?? Boolean(combatLabCurrent);

  function categoryProps(label: SidebarCategoryLabel) {
    return {
      label,
      open: !collapsedCategories[label],
      onToggle: () => {
        setCollapsedCategories((previous) => ({ ...previous, [label]: !previous[label] }));
      },
    };
  }

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
      breadcrumbs={<PlatformBreadcrumbs pathname={pathname} />}
      sidebar={
        <Sidebar aria-label="Main navigation">
          {signedInUser ? (
            <SidebarGovernorHeader handleLogout={handleLogout} />
          ) : (
            <SidebarHeader>
              <div className="px-2">
                <Image
                  src="https://cdn.rokbattles.com/assets/logo.svg"
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
            <SidebarCategory {...categoryProps("Community")}>
              <SidebarItem title="Battle Reports" href="/" current={pathname === "/"}>
                <Flame data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Battle Reports</SidebarLabel>
              </SidebarItem>
              <SidebarItem
                title="Olympian Arena"
                href="/olympian-arena"
                current={pathname === "/olympian-arena"}
              >
                <Trophy data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Olympian Arena</SidebarLabel>
              </SidebarItem>
            </SidebarCategory>
            <SidebarCategory {...categoryProps("Tools")}>
              <SidebarItem
                title="Loot Explorer"
                href="/loot-explorer"
                current={currentLocation(pathname, "/loot-explorer")}
              >
                <Gift data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Loot Explorer</SidebarLabel>
              </SidebarItem>
              <SidebarSubnav
                label="Combat Lab"
                href="/combat-lab"
                icon={<ChartPie data-slot="icon" aria-hidden="true" />}
                current={combatLabCurrent}
                open={showCombatLabSubnav}
                onToggle={() => setCombatLabOpen(!showCombatLabSubnav)}
              >
                <SidebarItem
                  title="Season of Conquest"
                  href="/combat-lab"
                  current={preSocCurrent ? false : combatLabCurrent}
                >
                  <Shield data-slot="icon" aria-hidden="true" />
                  <SidebarLabel>SoC</SidebarLabel>
                </SidebarItem>
                <SidebarItem
                  title="Pre-Season of Conquest"
                  href="/combat-lab/presoc"
                  current={preSocCurrent}
                >
                  <Swords data-slot="icon" aria-hidden="true" />
                  <SidebarLabel>Pre-SoC</SidebarLabel>
                </SidebarItem>
              </SidebarSubnav>
              <SidebarItem
                title="Territory Lab"
                href="/territory-lab"
                current={currentLocation(pathname, "/territory-lab")}
              >
                <MapIcon data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Territory Lab</SidebarLabel>
              </SidebarItem>
            </SidebarCategory>
            {signedInUser && activeGovernor && (
              <SidebarCategory {...categoryProps("My Data")}>
                {myDataLinks.map(({ label, href, icon: Icon }) => (
                  <SidebarItem
                    key={href}
                    title={label}
                    href={href}
                    current={currentLocation(pathname, href)}
                  >
                    <Icon data-slot="icon" aria-hidden="true" />
                    <SidebarLabel>{label}</SidebarLabel>
                  </SidebarItem>
                ))}
              </SidebarCategory>
            )}
            {signedInUser &&
              programLinks
                .filter(({ role }) => hasRole(signedInUser, role))
                .map(({ category, label, href, icon: Icon }) => (
                  <SidebarCategory key={category} {...categoryProps(category)}>
                    <SidebarItem
                      title={label}
                      href={href}
                      current={currentLocation(pathname, href)}
                    >
                      <Icon data-slot="icon" aria-hidden="true" />
                      <SidebarLabel>{label}</SidebarLabel>
                    </SidebarItem>
                  </SidebarCategory>
                ))}
            <SidebarSpacer />
            <SidebarSection>
              <SidebarItem
                title="Donate"
                href="https://github.com/sponsors/rokbattles"
                target="_blank"
                rel="noopener noreferrer"
                prefetch={false}
              >
                <Heart data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Donate</SidebarLabel>
              </SidebarItem>
              <SidebarItem
                title="Support"
                href="/discord"
                target="_blank"
                rel="noopener noreferrer"
                prefetch={false}
              >
                <CircleHelp data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Support</SidebarLabel>
              </SidebarItem>
              <SidebarItem
                title="Documentation"
                href="/docs"
                target="_blank"
                rel="noopener noreferrer"
                prefetch={false}
              >
                <BookOpen data-slot="icon" aria-hidden="true" />
                <SidebarLabel>Documentation</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
          </SidebarBody>
          {!user ? (
            <SidebarFooter>
              <SidebarItem title="Sign in" href="/proxy/v1/auth/discord/login" prefetch={false}>
                <LogIn data-slot="icon" aria-hidden="true" />
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
