import {
  ArrowRightStartOnRectangleIcon,
  ArrowTrendingUpIcon,
  ChevronDownIcon,
  Cog6ToothIcon,
  FireIcon,
  TrophyIcon,
  UsersIcon,
} from "@heroicons/react/16/solid";
import { getExtracted } from "next-intl/server";
import type { ReactNode } from "react";
import { signOut } from "@/actions/sign-out";
import { Avatar } from "@/components/ui/avatar";
import {
  Dropdown,
  DropdownButton,
  DropdownDivider,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/dropdown";
import {
  Navbar,
  NavbarDivider,
  NavbarItem,
  NavbarLabel,
  NavbarSection,
  NavbarSpacer,
} from "@/components/ui/navbar";
import {
  Sidebar,
  SidebarBody,
  SidebarHeader,
  SidebarItem,
  SidebarSection,
} from "@/components/ui/sidebar";
import { StackedLayout } from "@/components/ui/stacked-layout";
import { fetchCurrentUser } from "@/data/fetch-current-user";

export default async function AppLayout({ children }: { children: ReactNode }) {
  const t = await getExtracted();
  const currentUser = await fetchCurrentUser();

  return (
    <StackedLayout
      navbar={
        <Navbar>
          <NavbarItem className="max-lg:hidden" disabled>
            {t("ROK Battles")}
          </NavbarItem>
          <NavbarDivider className="max-lg:hidden" />
          <NavbarSection className="max-lg:hidden">
            <Dropdown>
              <DropdownButton aria-label="Open explore menu" as={NavbarItem}>
                <NavbarLabel>{t("Explore")}</NavbarLabel>
                <ChevronDownIcon />
              </DropdownButton>
              <DropdownMenu anchor="bottom start" className="min-w-48">
                <DropdownItem href="/explore">
                  <FireIcon />
                  <DropdownLabel>{t("Battle Reports")}</DropdownLabel>
                </DropdownItem>
                <DropdownItem href="/explore/arena">
                  <TrophyIcon />
                  <DropdownLabel>{t("Olympian Arena")}</DropdownLabel>
                </DropdownItem>
                <DropdownItem disabled>
                  <ArrowTrendingUpIcon />
                  <DropdownLabel>{t("Trends")}</DropdownLabel>
                </DropdownItem>
              </DropdownMenu>
            </Dropdown>
            <Dropdown>
              <DropdownButton aria-label="Open kingdom menu" as={NavbarItem}>
                <NavbarLabel>Kingdom</NavbarLabel>
                <ChevronDownIcon />
              </DropdownButton>
              <DropdownMenu anchor="bottom start" className="min-w-48">
                <DropdownItem disabled>
                  <UsersIcon />
                  <DropdownLabel>{t("Governors")}</DropdownLabel>
                </DropdownItem>
                <DropdownItem disabled>
                  <FireIcon />
                  <DropdownLabel>{t("KVKs")}</DropdownLabel>
                </DropdownItem>
                <DropdownDivider />
                <DropdownItem disabled>
                  <Cog6ToothIcon />
                  <DropdownLabel>{t("Management")}</DropdownLabel>
                </DropdownItem>
              </DropdownMenu>
            </Dropdown>
          </NavbarSection>
          <NavbarSpacer />
          <NavbarSection>
            {currentUser ? (
              <Dropdown>
                <DropdownButton aria-label="Open account menu" as={NavbarItem}>
                  <Avatar
                    alt={currentUser.globalName ?? currentUser.username}
                    square
                    src={currentUser.avatar}
                  />
                </DropdownButton>
                <DropdownMenu anchor="bottom end" className="min-w-48">
                  <DropdownItem disabled>
                    <Cog6ToothIcon />
                    <DropdownLabel>{t("Settings")}</DropdownLabel>
                  </DropdownItem>
                  <DropdownDivider />
                  <DropdownItem onClick={signOut}>
                    <ArrowRightStartOnRectangleIcon />
                    <DropdownLabel>{t("Sign out")}</DropdownLabel>
                  </DropdownItem>
                </DropdownMenu>
              </Dropdown>
            ) : (
              <NavbarItem href="/api/auth/login" prefetch={false}>
                <NavbarLabel>{t("Sign in")}</NavbarLabel>
              </NavbarItem>
            )}
          </NavbarSection>
        </Navbar>
      }
      sidebar={
        <Sidebar>
          <SidebarHeader>
            <SidebarItem disabled>{t("ROK Battles")}</SidebarItem>
          </SidebarHeader>
          <SidebarBody>
            <SidebarSection />
          </SidebarBody>
        </Sidebar>
      }
    >
      {children}
    </StackedLayout>
  );
}
