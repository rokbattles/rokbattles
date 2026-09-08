import { Flame } from "lucide-react";
import { Outlet, useMatch } from "react-router";
import { Navbar, NavbarItem } from "./ui/navbar";
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
  const isIndex = useMatch({ path: "/", end: true }) !== null;

  return (
    <SidebarLayout
      navbar={
        <Navbar aria-label="Site">
          <NavbarItem href="/">ROK Battles</NavbarItem>
        </Navbar>
      }
      sidebar={
        <Sidebar aria-label="Main navigation">
          <SidebarHeader>
            <SidebarItem href="/">
              <SidebarLabel>ROK Battles</SidebarLabel>
            </SidebarItem>
          </SidebarHeader>
          <SidebarBody>
            <SidebarSection>
              <SidebarHeading>Community</SidebarHeading>
              <SidebarItem href="/" current={isIndex}>
                <Flame aria-hidden="true" />
                <SidebarLabel>Battle Reports</SidebarLabel>
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
