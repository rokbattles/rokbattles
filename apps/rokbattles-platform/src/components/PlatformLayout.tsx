import { ArrowDownTrayIcon, FireIcon, QuestionMarkCircleIcon } from "@heroicons/react/16/solid";
import React from "react";
import { SidebarLayout } from "@/components/ui/layout/SidebarLayout";
import {
  Sidebar,
  SidebarBody,
  SidebarItem,
  SidebarLabel,
  SidebarSection,
  SidebarSpacer,
} from "@/components/ui/Sidebar";

export function PlatformLayout({ children }: { children: React.ReactNode }) {
  return (
    <SidebarLayout
      navbar={<React.Fragment />}
      sidebar={
        <Sidebar>
          <SidebarBody>
            <SidebarSection>
              <SidebarItem href="/">
                <FireIcon />
                <SidebarLabel>Explore</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
            <SidebarSpacer />
            <SidebarSection>
              <SidebarItem href="/discord" target="_blank" rel="noopenner noreferrer">
                <QuestionMarkCircleIcon />
                <SidebarLabel>Support</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/desktop-app" target="_blank" rel="noopenner noreferrer">
                <ArrowDownTrayIcon />
                <SidebarLabel>Desktop App</SidebarLabel>
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
