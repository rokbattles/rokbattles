import { Flame, FlaskConical, Gem, MapIcon, Swords } from "lucide-react";
import { Outlet, useLocation } from "react-router";
import { AppLogo } from "./app/logo";
import { Navbar } from "./ui/navbar";
import {
  Sidebar,
  SidebarBody,
  SidebarHeader,
  SidebarHeading,
  SidebarItem,
  SidebarLabel,
  SidebarSection,
} from "./ui/sidebar";
import { SidebarLayout } from "./ui/sidebar-layout";

export function AppLayout() {
  const { pathname } = useLocation();
  const section = pathname.toLowerCase().split("/")[2] ?? "";

  return (
    <SidebarLayout
      navbar={
        <Navbar aria-label="Site" className="pl-3">
          <AppLogo />
        </Navbar>
      }
      sidebar={
        <Sidebar aria-label="Main navigation">
          <SidebarHeader className="max-lg:hidden">
            <AppLogo />
          </SidebarHeader>
          <SidebarBody>
            <SidebarSection>
              <SidebarHeading>Community</SidebarHeading>
              <SidebarItem href="/app" current={section === ""}>
                <Flame aria-hidden="true" />
                <SidebarLabel>Battle Reports</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/app/olympian-arena" current={section === "olympian-arena"}>
                <Swords aria-hidden="true" />
                <SidebarLabel>Olympian Arena</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/app/combat-lab" current={section === "combat-lab"}>
                <FlaskConical aria-hidden="true" />
                <SidebarLabel>Combat Lab</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/app/loot-explorer" current={section === "loot-explorer"}>
                <Gem aria-hidden="true" />
                <SidebarLabel>Loot Explorer</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="/app/territory-lab" current={section === "territory-lab"}>
                <MapIcon aria-hidden="true" />
                <SidebarLabel>Territory Lab</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
          </SidebarBody>
        </Sidebar>
      }
    >
      <Outlet />
    </SidebarLayout>
  );
}
