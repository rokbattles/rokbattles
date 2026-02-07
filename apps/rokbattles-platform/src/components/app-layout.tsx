import { ChevronDownIcon } from "@heroicons/react/16/solid";
import { getExtracted } from "next-intl/server";
import type { ReactNode } from "react";
import {
  Dropdown,
  DropdownButton,
  DropdownItem,
  DropdownMenu,
} from "@/components/ui/dropdown";
import {
  Navbar,
  NavbarDivider,
  NavbarItem,
  NavbarLabel,
  NavbarSection,
} from "@/components/ui/navbar";
import {
  Sidebar,
  SidebarBody,
  SidebarHeader,
  SidebarItem,
  SidebarSection,
} from "@/components/ui/sidebar";
import { StackedLayout } from "@/components/ui/stacked-layout";

export default async function AppLayout({ children }: { children: ReactNode }) {
  const t = await getExtracted();

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
              <DropdownButton as={NavbarItem}>
                <NavbarLabel>{t("Explore")}</NavbarLabel>
                <ChevronDownIcon />
              </DropdownButton>
              <DropdownMenu anchor="bottom start" className="min-w-48">
                <DropdownItem href="/explore">
                  {t("Battle Reports")}
                </DropdownItem>
                <DropdownItem href="/explore/arena">
                  {t("Olympian Arena")}
                </DropdownItem>
              </DropdownMenu>
            </Dropdown>
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
