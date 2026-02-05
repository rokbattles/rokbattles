import { useExtracted } from "next-intl";
import type { ReactNode } from "react";
import {
  Navbar,
  NavbarDivider,
  NavbarItem,
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

export default function AppLayout({ children }: { children: ReactNode }) {
  const t = useExtracted();

  return (
    <StackedLayout
      navbar={
        <Navbar>
          <NavbarItem className="max-lg:hidden" disabled>
            {t("ROK Battles")}
          </NavbarItem>
          <NavbarDivider className="max-lg:hidden" />
          <NavbarSection className="max-lg:hidden" />
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
