"use client";

import {
  ChartNoAxesCombinedIcon,
  CircleQuestionMarkIcon,
  DownloadIcon,
  SwordsIcon,
} from "lucide-react";
import React from "react";
import { Badge } from "@/components/ui/badge";
import { Heading } from "@/components/ui/heading";
import { Link } from "@/components/ui/link";
import {
  Sidebar,
  SidebarBody,
  SidebarHeader,
  SidebarItem,
  SidebarLabel,
  SidebarSection,
  SidebarSpacer,
} from "@/components/ui/sidebar";
import { SidebarLayout } from "@/components/ui/sidebar-layout";
import { usePathname } from "@/i18n/navigation";

export function ExploreLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();

  return (
    <SidebarLayout
      navbar={<React.Fragment />}
      sidebar={
        <Sidebar>
          <SidebarHeader>
            <Link href="/">
              <Heading className="text-center">ROK Battles</Heading>
            </Link>
          </SidebarHeader>
          <SidebarBody>
            <SidebarSection>
              <SidebarItem href="/explore" current={pathname === "/explore"}>
                <SwordsIcon data-slot="lucide" />
                <SidebarLabel>Battle Reports</SidebarLabel>
              </SidebarItem>
              <SidebarItem>
                <ChartNoAxesCombinedIcon data-slot="lucide" />
                <SidebarLabel>Trends</SidebarLabel>
                <Badge>Soon</Badge>
              </SidebarItem>
            </SidebarSection>
            <SidebarSpacer />
            <SidebarSection>
              <SidebarItem
                href="https://discord.gg/G33SzQgx6d"
                target="_blank"
                rel="noopener noreferrer"
              >
                <CircleQuestionMarkIcon data-slot="lucide" />
                <SidebarLabel>Support</SidebarLabel>
              </SidebarItem>
              <SidebarItem
                href="https://github.com/rokbattles/rokbattles/releases"
                target="_blank"
                rel="noopener noreferrer"
              >
                <DownloadIcon data-slot="lucide" />
                <SidebarLabel>Download App</SidebarLabel>
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
