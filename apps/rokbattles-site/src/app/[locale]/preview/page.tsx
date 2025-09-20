import {
  ChartNoAxesCombinedIcon,
  ChevronUpIcon,
  CircleQuestionMarkIcon,
  HouseIcon,
  SwordsIcon,
} from "lucide-react";
import { Avatar } from "@/components/ui/avatar";
import {
  Dropdown,
  DropdownButton,
  DropdownDivider,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/dropdown";
import { Navbar, NavbarItem, NavbarSection, NavbarSpacer } from "@/components/ui/navbar";
import {
  Sidebar,
  SidebarBody,
  SidebarFooter,
  SidebarHeader,
  SidebarItem,
  SidebarLabel,
  SidebarSection,
  SidebarSpacer,
} from "@/components/ui/sidebar";
import { SidebarLayout } from "@/components/ui/sidebar-layout";
import { Text } from "@/components/ui/text";

function AccountDropdownMenu({ anchor }: { anchor: "top start" | "bottom end" }) {
  return (
    <DropdownMenu className="min-w-64" anchor={anchor}>
      <DropdownItem href="#">
        <DropdownLabel>My account</DropdownLabel>
      </DropdownItem>
      <DropdownDivider />
      <DropdownItem href="#">
        <DropdownLabel>Privacy policy</DropdownLabel>
      </DropdownItem>
      <DropdownItem href="#">
        <DropdownLabel>Share feedback</DropdownLabel>
      </DropdownItem>
      <DropdownDivider />
      <DropdownItem href="#">
        <DropdownLabel>Sign out</DropdownLabel>
      </DropdownItem>
    </DropdownMenu>
  );
}

export default function Page() {
  return (
    <SidebarLayout
      navbar={
        <Navbar>
          <NavbarSpacer />
          <NavbarSection>
            <Dropdown>
              <DropdownButton as={NavbarItem}>
                <Avatar initials="G" square />
              </DropdownButton>
              <AccountDropdownMenu anchor="bottom end" />
            </Dropdown>
          </NavbarSection>
        </Navbar>
      }
      sidebar={
        <Sidebar>
          <SidebarHeader>
            <SidebarItem>
              <SidebarLabel>ROK Battles</SidebarLabel>
            </SidebarItem>
          </SidebarHeader>
          <SidebarBody>
            <SidebarSection>
              <SidebarItem href="#">
                <HouseIcon data-slot="icon" />
                <SidebarLabel>Home</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="#">
                <SwordsIcon data-slot="icon" />
                <SidebarLabel>Battle Reports</SidebarLabel>
              </SidebarItem>
              <SidebarItem href="#">
                <ChartNoAxesCombinedIcon data-slot="icon" />
                <SidebarLabel>Trends</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
            <SidebarSpacer />
            <SidebarSection>
              <SidebarItem className="#">
                <CircleQuestionMarkIcon data-slot="icon" />
                <SidebarLabel>Support</SidebarLabel>
              </SidebarItem>
            </SidebarSection>
          </SidebarBody>
          <SidebarFooter className="max-lg:hidden">
            <Dropdown>
              <DropdownButton as={SidebarItem}>
                <span className="flex min-w-0 items-center gap-3">
                  <Avatar initials="G" className="size-10" square alt="" />
                  <span className="min-w-0">
                    <span className="block truncate text-sm/5 font-medium text-zinc-950 dark:text-white">
                      Grigvar
                    </span>
                  </span>
                </span>
                <ChevronUpIcon data-slot="icon" />
              </DropdownButton>
              <AccountDropdownMenu anchor="top start" />
            </Dropdown>
          </SidebarFooter>
        </Sidebar>
      }
    >
      <Text>Main content here</Text>
    </SidebarLayout>
  );
}
